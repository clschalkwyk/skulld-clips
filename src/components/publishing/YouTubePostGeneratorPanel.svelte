<script lang="ts">
  import { onMount } from "svelte";

  import type {
    AiModelOption,
    AiPostProvider,
    AiProviderCredentialStatus,
    AppError,
    ClipEventKind,
    YouTubePostGenerationSource,
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
  import {
    clearAiProviderApiKey,
    generateAiYouTubePost,
    getAiProviderCredentialStatuses,
    listAiProviderModels,
    saveAiProviderApiKey,
  } from "../../services/tauri";

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
  let generationSource = $state<YouTubePostGenerationSource>("local");
  let credentialStatuses = $state<AiProviderCredentialStatus[]>([]);
  let modelsByProvider = $state<Record<AiPostProvider, AiModelOption[]>>({
    openai: [],
    openrouter: [],
  });
  let selectedModels = $state<Record<AiPostProvider, string>>({
    openai: "",
    openrouter: "",
  });
  let apiKeyInput = $state("");
  let providerLoading = $state(false);
  let generationLoading = $state(false);
  let providerError = $state<AppError | null>(null);
  let providerNotice = $state<string | null>(null);
  let errors = $derived(validateYouTubePostBrief(brief));
  let selectedProvider = $derived(
    generationSource === "local" ? null : generationSource,
  );
  let selectedCredentialStatus = $derived(
    selectedProvider
      ? (credentialStatuses.find(
          (status) => status.provider === selectedProvider,
        ) ?? null)
      : null,
  );
  let selectedProviderModels = $derived(
    selectedProvider ? modelsByProvider[selectedProvider] : [],
  );
  let selectedModel = $derived(
    selectedProvider ? selectedModels[selectedProvider] : "",
  );
  let canGenerate = $derived(
    Object.keys(errors).length === 0 &&
      !generationLoading &&
      (generationSource === "local" ||
        (Boolean(selectedCredentialStatus?.configured) &&
          selectedModel.length > 0)),
  );
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
    void loadCredentialStatuses();
  });

  async function generate(): Promise<void> {
    if (!canGenerate) {
      return;
    }
    providerError = null;
    generationLoading = true;
    try {
      draft =
        generationSource === "local"
          ? generateYouTubePost(brief)
          : await generateAiYouTubePost(
              generationSource,
              selectedModels[generationSource],
              brief,
            );
    } catch (error) {
      providerError = error as AppError;
    } finally {
      generationLoading = false;
    }
    copyStatus = null;
    copyError = null;
  }

  async function loadCredentialStatuses(): Promise<void> {
    providerLoading = true;
    providerError = null;
    try {
      credentialStatuses = await getAiProviderCredentialStatuses();
    } catch (error) {
      providerError = error as AppError;
    } finally {
      providerLoading = false;
    }
  }

  async function handleSourceChange(): Promise<void> {
    apiKeyInput = "";
    providerError = null;
    providerNotice = null;
    if (
      generationSource !== "local" &&
      credentialStatuses.find(
        (status) => status.provider === generationSource,
      )?.configured &&
      modelsByProvider[generationSource].length === 0
    ) {
      await loadModels(generationSource);
    }
  }

  async function saveApiKey(): Promise<void> {
    if (!selectedProvider || apiKeyInput.length < 20) {
      return;
    }
    providerLoading = true;
    providerError = null;
    providerNotice = null;
    try {
      const status = await saveAiProviderApiKey(
        selectedProvider,
        apiKeyInput,
      );
      updateCredentialStatus(status);
      apiKeyInput = "";
      const modelsLoaded = await loadModels(selectedProvider, false);
      if (modelsLoaded) {
        providerNotice = `${providerLabel(selectedProvider)} key saved securely and model list loaded.`;
      }
    } catch (error) {
      providerError = error as AppError;
    } finally {
      providerLoading = false;
    }
  }

  async function clearApiKey(): Promise<void> {
    if (!selectedProvider) {
      return;
    }
    providerLoading = true;
    providerError = null;
    providerNotice = null;
    try {
      const status = await clearAiProviderApiKey(selectedProvider);
      updateCredentialStatus(status);
      modelsByProvider = {
        ...modelsByProvider,
        [selectedProvider]: [],
      };
      selectedModels = {
        ...selectedModels,
        [selectedProvider]: "",
      };
      providerNotice = `${providerLabel(selectedProvider)} key removed from the credential store.`;
    } catch (error) {
      providerError = error as AppError;
    } finally {
      providerLoading = false;
    }
  }

  async function loadModels(
    provider: AiPostProvider,
    ownLoadingState = true,
  ): Promise<boolean> {
    if (ownLoadingState) {
      providerLoading = true;
      providerError = null;
      providerNotice = null;
    }
    try {
      const models = await listAiProviderModels(provider);
      modelsByProvider = {
        ...modelsByProvider,
        [provider]: models,
      };
      const currentSelection = selectedModels[provider];
      selectedModels = {
        ...selectedModels,
        [provider]: models.some((model) => model.id === currentSelection)
          ? currentSelection
          : (models[0]?.id ?? ""),
      };
      if (models.length === 0) {
        providerNotice = `${providerLabel(provider)} returned no compatible text-generation models.`;
      }
      return models.length > 0;
    } catch (error) {
      providerError = error as AppError;
      return false;
    } finally {
      if (ownLoadingState) {
        providerLoading = false;
      }
    }
  }

  function updateCredentialStatus(
    nextStatus: AiProviderCredentialStatus,
  ): void {
    credentialStatuses = [
      ...credentialStatuses.filter(
        (status) => status.provider !== nextStatus.provider,
      ),
      nextStatus,
    ];
  }

  function selectModel(event: Event): void {
    if (!selectedProvider) {
      return;
    }
    selectedModels = {
      ...selectedModels,
      [selectedProvider]: (event.currentTarget as HTMLSelectElement).value,
    };
  }

  function providerLabel(provider: AiPostProvider): string {
    return provider === "openai" ? "OpenAI" : "OpenRouter";
  }

  function providerKeyPlaceholder(provider: AiPostProvider): string {
    return provider === "openai" ? "sk-…" : "sk-or-v1-…";
  }

  function formatContextLength(contextLength: number | null): string {
    if (!contextLength) {
      return "";
    }
    return ` · ${Intl.NumberFormat("en", { notation: "compact" }).format(contextLength)} context`;
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
          <span>
            {generationSource === "local"
              ? "Local generation · no upload"
              : `${providerLabel(generationSource)} · brief only`}
          </span>
        </div>

        <p class="post-explainer">
          Clip Forge seeds this brief from the project, applied moment, and hook
          caption. Correct the details before generating—SEO copy should describe
          the real video, not invent it.
        </p>

        <div class="post-provider-panel" aria-labelledby="post-provider-heading">
          <div class="post-provider-heading">
            <div>
              <p class="section-label">Generation engine</p>
              <h4 id="post-provider-heading">Choose where the copy is written</h4>
            </div>
            {#if selectedProvider}
              <span
                class="post-provider-status"
                class:configured={selectedCredentialStatus?.configured}
              >
                {selectedCredentialStatus?.configured
                  ? "Key saved"
                  : "Key required"}
              </span>
            {/if}
          </div>

          <label>
            <span>Generation source</span>
            <select
              bind:value={generationSource}
              onchange={() => void handleSourceChange()}
              disabled={providerLoading || generationLoading}
            >
              <option value="local">Local template</option>
              <option value="openai">OpenAI API</option>
              <option value="openrouter">OpenRouter API</option>
            </select>
          </label>

          {#if selectedProvider}
            <p class="post-provider-disclosure">
              Only this factual content brief is sent to
              {providerLabel(selectedProvider)}. Clip Forge never sends the source
              video, project file, private paths, or YouTube credentials.
            </p>

            <div class="post-provider-key-row">
              <label>
                <span>{providerLabel(selectedProvider)} API key</span>
                <input
                  type="password"
                  autocomplete="off"
                  spellcheck="false"
                  placeholder={providerKeyPlaceholder(selectedProvider)}
                  bind:value={apiKeyInput}
                  disabled={providerLoading || generationLoading}
                />
                <small>
                  Saving validates the key, then stores it in the operating-system
                  credential store.
                </small>
              </label>
              <button
                class="secondary-button"
                type="button"
                onclick={() => void saveApiKey()}
                disabled={providerLoading ||
                  generationLoading ||
                  apiKeyInput.length < 20}
              >
                {providerLoading ? "Checking…" : "Save key"}
              </button>
            </div>

            {#if selectedCredentialStatus?.configured}
              <div class="post-provider-model-row">
                <label>
                  <span>{providerLabel(selectedProvider)} model</span>
                  <select
                    value={selectedModel}
                    onchange={selectModel}
                    disabled={providerLoading ||
                      generationLoading ||
                      selectedProviderModels.length === 0}
                  >
                    {#if selectedProviderModels.length === 0}
                      <option value="">No models loaded</option>
                    {:else}
                      {#each selectedProviderModels as model (model.id)}
                        <option value={model.id}>
                          {model.name}{formatContextLength(model.contextLength)}
                        </option>
                      {/each}
                    {/if}
                  </select>
                  <small>
                    The list comes from the provider and reflects models available
                    to this key.
                  </small>
                </label>
                <div class="post-provider-actions">
                  <button
                    class="secondary-button"
                    type="button"
                    onclick={() => void loadModels(selectedProvider)}
                    disabled={providerLoading || generationLoading}
                  >
                    Refresh models
                  </button>
                  <button
                    class="text-button"
                    type="button"
                    onclick={() => void clearApiKey()}
                    disabled={providerLoading || generationLoading}
                  >
                    Remove saved key
                  </button>
                </div>
              </div>
            {/if}

            {#if providerNotice}
              <p class="post-provider-notice" role="status">{providerNotice}</p>
            {/if}
            {#if providerError}
              <div class="post-provider-error" role="alert">
                <strong>{providerError.message}</strong>
                {#if providerError.safeDetail}
                  <span>{providerError.safeDetail}</span>
                {/if}
                <code>{providerError.code}</code>
              </div>
            {/if}
          {/if}
        </div>

        {#if generationSource === "local" && providerError}
          <div class="post-provider-error" role="alert">
            <strong>{providerError.message}</strong>
            {#if providerError.safeDetail}
              <span>{providerError.safeDetail}</span>
            {/if}
            <code>{providerError.code}</code>
          </div>
        {/if}

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
            {#if generationLoading}
              Generating with {selectedProvider
                ? providerLabel(selectedProvider)
                : "local template"}…
            {:else if draft}
              Regenerate post
            {:else}
              Generate title + description
            {/if}
          </button>
          <span>
            {sourceFilename} ·
            {generationSource === "local"
              ? "offline"
              : `${providerLabel(generationSource)} model`}
          </span>
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
      <span>
        Keys: OS credential store · descriptions: 5,000 maximum · 3 hashtags
      </span>
    </footer>
  </div>
</div>
