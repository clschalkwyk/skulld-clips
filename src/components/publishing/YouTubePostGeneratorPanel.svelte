<script lang="ts">
  import { onMount } from "svelte";

  import type {
    ClipEventKind,
    YouTubePostBrief,
    YouTubePostDraft,
    YouTubePostMomentType,
  } from "../../../contracts/types";
  import {
    analyzeYouTubePost,
    defaultYouTubeSearchPhrase,
    generateYouTubePost,
    validateYouTubePostBrief,
    YOUTUBE_DESCRIPTION_LIMIT,
    YOUTUBE_TITLE_LIMIT,
  } from "../../services/youtube-post-generator";

  interface Props {
    projectName: string;
    sourceFilename: string;
    trimDurationMs: number;
    detectedMomentKind: ClipEventKind | null;
    brief: YouTubePostBrief;
    draft: YouTubePostDraft | null;
    onClose: () => void;
  }

  let {
    projectName,
    sourceFilename,
    trimDurationMs,
    detectedMomentKind,
    brief = $bindable(),
    draft = $bindable(),
    onClose,
  }: Props = $props();

  let closeButton = $state<HTMLButtonElement>();
  let copyStatus = $state<string | null>(null);
  let copyError = $state<string | null>(null);
  let errors = $derived(validateYouTubePostBrief(brief));
  let canGenerate = $derived(Object.keys(errors).length === 0);
  let checks = $derived(
    draft
      ? analyzeYouTubePost(
          draft.title,
          draft.description,
          brief.primarySearchPhrase,
        )
      : null,
  );

  onMount(() => {
    closeButton?.focus();
  });

  function generate(): void {
    if (!canGenerate) {
      return;
    }
    draft = generateYouTubePost(brief);
    copyStatus = null;
    copyError = null;
  }

  function useSuggestedSearchPhrase(): void {
    brief.primarySearchPhrase = defaultYouTubeSearchPhrase(
      brief.game,
      brief.momentType,
    );
    draft = null;
  }

  function selectTitle(title: string): void {
    if (!draft) {
      return;
    }
    draft.title = title;
    copyStatus = null;
  }

  async function copyText(
    value: string,
    successMessage: string,
  ): Promise<void> {
    copyStatus = null;
    copyError = null;
    try {
      if (!navigator.clipboard?.writeText) {
        throw new Error("Clipboard access is unavailable in this build.");
      }
      await navigator.clipboard.writeText(value);
      copyStatus = successMessage;
    } catch {
      copyError =
        "Clip Forge could not write to the clipboard. Select the text and copy it manually.";
    }
  }

  function momentLabel(value: YouTubePostMomentType): string {
    switch (value) {
      case "completion":
        return "Dungeon completion";
      case "death":
        return "Death moment";
      case "bossEncounter":
        return "Boss fight";
      case "buildShowcase":
        return "Build showcase";
      case "guide":
        return "Quick guide";
      case "gameplayHighlight":
        return "Gameplay highlight";
    }
  }

  function detectedMomentLabel(value: ClipEventKind | null): string {
    return value ? `${momentLabel(value)} detected` : "No detected moment applied";
  }

  function formatDuration(durationMs: number): string {
    const seconds = Math.max(0, Math.round(durationMs / 1_000));
    const minutes = Math.floor(seconds / 60);
    return `${minutes}:${(seconds % 60).toString().padStart(2, "0")}`;
  }
</script>

<div class="post-backdrop" role="presentation">
  <div
    class="post-dialog"
    role="dialog"
    aria-modal="true"
    aria-labelledby="youtube-post-heading"
  >
    <header class="post-header">
      <div>
        <p class="section-label">YouTube publishing copy</p>
        <h2 id="youtube-post-heading">Post generator</h2>
      </div>
      <button
        class="text-button"
        type="button"
        aria-label="Close YouTube post generator"
        onclick={onClose}
        bind:this={closeButton}
      >
        Close
      </button>
    </header>

    <div class="post-context-bar">
      <div>
        <span>Current project</span>
        <strong>{projectName}</strong>
      </div>
      <div>
        <span>Selected clip</span>
        <strong>{formatDuration(trimDurationMs)}</strong>
      </div>
      <div>
        <span>Content signal</span>
        <strong>{detectedMomentLabel(detectedMomentKind)}</strong>
      </div>
    </div>

    <div class="post-workspace">
      <section class="post-brief" aria-labelledby="post-brief-heading">
        <div class="post-section-heading">
          <div>
            <p class="section-label">Content brief</p>
            <h3 id="post-brief-heading">Tell viewers what actually happens</h3>
          </div>
          <span>Local generation · no upload</span>
        </div>

        <p class="post-explainer">
          Clip Forge seeds this brief from the project, applied moment, and hook
          caption. Correct the details before generating—SEO copy should describe
          the real video, not invent it.
        </p>

        <div class="post-form-grid">
          <label>
            <span>Game</span>
            <input
              type="text"
              maxlength="60"
              bind:value={brief.game}
              aria-invalid={Boolean(errors.game)}
            />
            {#if errors.game}<small class="field-error">{errors.game}</small>{/if}
          </label>

          <label>
            <span>YouTube format</span>
            <select bind:value={brief.format}>
              <option value="short">YouTube Short</option>
              <option value="video">YouTube video</option>
            </select>
          </label>

          <label>
            <span>Moment type</span>
            <select bind:value={brief.momentType}>
              <option value="bossEncounter">Boss fight</option>
              <option value="completion">Dungeon completion</option>
              <option value="death">Death moment</option>
              <option value="buildShowcase">Build showcase</option>
              <option value="guide">Quick guide</option>
              <option value="gameplayHighlight">Gameplay highlight</option>
            </select>
          </label>

          <label class="post-search-field">
            <span>Primary search phrase</span>
            <div class="post-input-action">
              <input
                type="text"
                maxlength="80"
                placeholder="Diablo 4 Butcher fight"
                bind:value={brief.primarySearchPhrase}
                aria-invalid={Boolean(errors.primarySearchPhrase)}
              />
              <button
                class="text-button"
                type="button"
                onclick={useSuggestedSearchPhrase}
              >
                Suggest
              </button>
            </div>
            {#if errors.primarySearchPhrase}
              <small class="field-error">{errors.primarySearchPhrase}</small>
            {:else}
              <small>Use the phrase a real viewer would search for.</small>
            {/if}
          </label>

          <label class="post-wide-field">
            <span>What happens in this clip?</span>
            <textarea
              rows="4"
              maxlength="280"
              placeholder="The Butcher ambushed my Whirlwind Barbarian and the fight came down to the final hit."
              bind:value={brief.contentSummary}
              aria-invalid={Boolean(errors.contentSummary)}
            ></textarea>
            <small class:field-error={Boolean(errors.contentSummary)}>
              {errors.contentSummary ??
                `${brief.contentSummary.length}/280 · factual, specific, one clear moment`}
            </small>
          </label>

          <label class="post-wide-field">
            <span>Supporting keywords</span>
            <input
              type="text"
              maxlength="240"
              placeholder="Whirlwind Barbarian, Season 14, boss encounter"
              bind:value={brief.supportingKeywords}
              aria-invalid={Boolean(errors.supportingKeywords)}
            />
            <small class:field-error={Boolean(errors.supportingKeywords)}>
              {errors.supportingKeywords ??
                "Optional · comma-separated · only terms visible or relevant in the clip"}
            </small>
          </label>

          <label class="post-wide-field">
            <span>Call to action</span>
            <input
              type="text"
              maxlength="240"
              bind:value={brief.callToAction}
              aria-invalid={Boolean(errors.callToAction)}
            />
            {#if errors.callToAction}
              <small class="field-error">{errors.callToAction}</small>
            {/if}
          </label>
        </div>

        <div class="post-generate-row">
          <button
            class="primary-button"
            type="button"
            onclick={generate}
            disabled={!canGenerate}
          >
            {draft ? "Regenerate post" : "Generate title + description"}
          </button>
          <span>{sourceFilename}</span>
        </div>
      </section>

      <section class="post-output" aria-labelledby="post-output-heading">
        <div class="post-section-heading">
          <div>
            <p class="section-label">Editable output</p>
            <h3 id="post-output-heading">YouTube metadata</h3>
          </div>
          {#if draft}<span>Choose, edit, then copy</span>{/if}
        </div>

        {#if !draft}
          <div class="post-empty" role="status">
            <strong>Your post copy will appear here.</strong>
            <p>
              Complete the content brief to generate three honest title angles
              and one search-aware description.
            </p>
          </div>
        {:else}
          <div class="post-title-options" aria-label="Generated title options">
            {#each draft.titleOptions as option (option.id)}
              <button
                class:selected={draft.title === option.title}
                type="button"
                onclick={() => selectTitle(option.title)}
              >
                <span>{option.label}</span>
                <strong>{option.title}</strong>
                <small>{[...option.title].length}/{YOUTUBE_TITLE_LIMIT}</small>
              </button>
            {/each}
          </div>

          <label class="post-output-field">
            <span>
              Selected title
              <small>{checks?.titleCharacters ?? 0}/{YOUTUBE_TITLE_LIMIT}</small>
            </span>
            <input
              type="text"
              maxlength={YOUTUBE_TITLE_LIMIT}
              bind:value={draft.title}
            />
          </label>

          <label class="post-output-field">
            <span>
              Description
              <small>
                {checks?.descriptionCharacters ?? 0}/{YOUTUBE_DESCRIPTION_LIMIT}
              </small>
            </span>
            <textarea
              rows="11"
              maxlength={YOUTUBE_DESCRIPTION_LIMIT}
              bind:value={draft.description}
            ></textarea>
          </label>

          {#if checks}
            <div class="post-checks" aria-label="YouTube metadata checks">
              <span class:pass={checks.titleWithinLimit}>
                Title ≤ 100
              </span>
              <span class:pass={checks.descriptionWithinLimit}>
                Description ≤ 5,000
              </span>
              <span class:pass={checks.searchPhraseInTitle}>
                Search phrase in title
              </span>
              <span class:pass={checks.searchPhraseInOpeningDescription}>
                Search phrase opens description
              </span>
              <span class:pass={checks.hashtagCount <= 3}>
                {checks.hashtagCount}/3 hashtags
              </span>
            </div>
          {/if}

          <div class="post-copy-actions">
            <button
              class="secondary-button"
              type="button"
              onclick={() => copyText(draft?.title ?? "", "Title copied.")}
            >
              Copy title
            </button>
            <button
              class="secondary-button"
              type="button"
              onclick={() =>
                copyText(draft?.description ?? "", "Description copied.")}
            >
              Copy description
            </button>
            <button
              class="primary-button"
              type="button"
              onclick={() =>
                copyText(
                  `${draft?.title ?? ""}\n\n${draft?.description ?? ""}`,
                  "Title and description copied.",
                )}
            >
              Copy full post
            </button>
          </div>

          {#if copyStatus}
            <p class="post-copy-status" role="status">{copyStatus}</p>
          {/if}
          {#if copyError}
            <p class="field-error post-copy-status" role="alert">{copyError}</p>
          {/if}
        {/if}
      </section>
    </div>

    <footer class="post-footer">
      <span>Titles: 100 characters maximum</span>
      <span>Descriptions: 5,000 characters maximum · 3 relevant hashtags</span>
    </footer>
  </div>
</div>
