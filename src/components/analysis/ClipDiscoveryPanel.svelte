<script lang="ts">
  import { onMount, tick } from "svelte";

  import type { ClipCandidate, ClipEventKind } from "../../../contracts/types";
  import type { ClipAnalysisState } from "../../services/clip-analysis-state";
  import {
    applyMomentExtractionWindow,
    clipAnalysisIsActive,
    validateMomentExtractionWindow,
  } from "../../services/clip-analysis-state";
  import { formatTimelineTime } from "../../services/timeline";

  interface Props {
    sourceFilename: string;
    sourceUrl: string | null;
    sourceDurationMs: number;
    analysisState: ClipAnalysisState;
    momentExtractStartTimeSeconds: number;
    momentExtractEndTimeSeconds: number;
    onStart: () => Promise<void>;
    onCancel: () => Promise<void>;
    onApply: (candidate: ClipCandidate) => void;
    onClose: () => void;
  }

  let {
    sourceFilename,
    sourceUrl,
    sourceDurationMs,
    analysisState,
    momentExtractStartTimeSeconds = $bindable(),
    momentExtractEndTimeSeconds = $bindable(),
    onStart,
    onCancel,
    onApply,
    onClose,
  }: Props = $props();

  let closeButton = $state<HTMLButtonElement>();
  let reviewVideo = $state<HTMLVideoElement>();
  let reviewedCandidate = $state<ClipCandidate | null>(null);
  let momentExtractionWindow = $derived({
    momentExtractStartTimeMs: momentExtractStartTimeSeconds * 1_000,
    momentExtractEndTimeMs: momentExtractEndTimeSeconds * 1_000,
  });
  let extractionWindowError = $derived(
    validateMomentExtractionWindow(momentExtractionWindow),
  );

  onMount(() => {
    closeButton?.focus();
  });

  function kindLabel(kind: ClipEventKind): string {
    switch (kind) {
      case "completion":
        return "Completion / title screen";
      case "death":
        return "Player death";
      case "bossEncounter":
        return "Boss encounter";
    }
  }

  function confidenceLabel(confidence: number): string {
    return `${Math.round(confidence * 100)}% confidence`;
  }

  async function review(candidate: ClipCandidate): Promise<void> {
    reviewedCandidate = candidate;
    await tick();
    seekReviewVideo();
  }

  function seekReviewVideo(): void {
    if (!reviewedCandidate || !reviewVideo) {
      return;
    }
    reviewVideo.pause();
    reviewVideo.currentTime = reviewedCandidate.eventMs / 1_000;
  }

  function configuredCandidate(candidate: ClipCandidate): ClipCandidate {
    return applyMomentExtractionWindow(
      candidate,
      sourceDurationMs,
      momentExtractionWindow,
    );
  }
</script>

<div class="analysis-backdrop" role="presentation">
  <div
    class="analysis-dialog"
    role="dialog"
    aria-modal="true"
    aria-labelledby="analysis-heading"
  >
    <header class="analysis-header">
      <div>
        <p class="section-label">Diablo IV scene discovery</p>
        <h2 id="analysis-heading">Find clip moments</h2>
      </div>
      <button
        class="text-button"
        type="button"
        aria-label="Close clip discovery"
        onclick={onClose}
        bind:this={closeButton}
      >
        Close
      </button>
    </header>

    <div class="analysis-intro">
      <div>
        <strong>{sourceFilename}</strong>
        <p>
          Scan locally for completion titles, player-death screens and persistent
          wide boss health bars. Every result is a suggestion for review.
        </p>
      </div>
      <button
        class="primary-button"
        type="button"
        onclick={onStart}
        disabled={clipAnalysisIsActive(analysisState)}
      >
        {analysisState.status === "idle" ? "Scan source" : "Scan again"}
      </button>
    </div>

    <section
      class="analysis-extraction-window"
      aria-labelledby="analysis-extraction-window-heading"
    >
      <div class="analysis-extraction-heading">
        <div>
          <p class="section-label">Moment extraction window</p>
          <h3 id="analysis-extraction-window-heading">Keep the lead-up and aftermath</h3>
        </div>
        <span>Defaults: 15s before · 5s after</span>
      </div>
      <div class="analysis-extraction-fields">
        <label for="moment-extract-start-time">
          <span>Before moment</span>
          <span class="analysis-number-input">
            <input
              id="moment-extract-start-time"
              name="moment-extract-start-time"
              type="number"
              min="0"
              max="300"
              step="0.5"
              bind:value={momentExtractStartTimeSeconds}
              aria-describedby="moment-extraction-help moment-extraction-error"
            />
            <small>seconds</small>
          </span>
        </label>
        <label for="moment-extract-end-time">
          <span>After moment</span>
          <span class="analysis-number-input">
            <input
              id="moment-extract-end-time"
              name="moment-extract-end-time"
              type="number"
              min="0"
              max="300"
              step="0.5"
              bind:value={momentExtractEndTimeSeconds}
              aria-describedby="moment-extraction-help moment-extraction-error"
            />
            <small>seconds</small>
          </span>
        </label>
      </div>
      <p id="moment-extraction-help">
        Suggested ranges update immediately. Detection does not need to run again.
      </p>
      {#if extractionWindowError}
        <p id="moment-extraction-error" class="field-error" role="alert">
          {extractionWindowError}
        </p>
      {/if}
    </section>

    {#if clipAnalysisIsActive(analysisState)}
      <section class="analysis-progress" aria-live="polite">
        <div>
          <strong>Analyzing gameplay frames…</strong>
          <span>
            {formatTimelineTime(analysisState.analyzedMs)} of
            {formatTimelineTime(analysisState.totalMs)}
          </span>
        </div>
        <progress max="1" value={analysisState.progress}></progress>
        <button
          class="secondary-button"
          type="button"
          onclick={onCancel}
          disabled={analysisState.cancelRequested ||
            analysisState.jobId === null}
        >
          {analysisState.cancelRequested ? "Cancelling…" : "Cancel scan"}
        </button>
      </section>
    {:else if analysisState.status === "error" && analysisState.error}
      <section class="analysis-message analysis-failed" role="alert">
        <strong>{analysisState.error.message}</strong>
        {#if analysisState.error.safeDetail}<p>{analysisState.error.safeDetail}</p>{/if}
        <code>{analysisState.error.code}</code>
      </section>
    {:else if analysisState.status === "cancelled"}
      <section class="analysis-message" role="status">
        <strong>Scan cancelled.</strong>
        <p>The current project and trim range were not changed.</p>
      </section>
    {:else if analysisState.status === "completed" &&
      analysisState.candidates.length === 0}
      <section class="analysis-message" role="status">
        <strong>No high-confidence moments found.</strong>
        <p>
          The detector did not see a persistent Diablo IV UI signature. Keep
          trimming manually or scan another source.
        </p>
      </section>
    {:else if analysisState.candidates.length > 0}
      <section class="analysis-results" aria-labelledby="analysis-results-heading">
        <div class="analysis-results-heading">
          <div>
            <p class="section-label">Review required</p>
            <h3 id="analysis-results-heading">
              {analysisState.candidates.length}
              {analysisState.candidates.length === 1 ? "moment" : "moments"} found
            </h3>
          </div>
          <span>Nothing has been applied yet</span>
        </div>

        {#if reviewedCandidate && sourceUrl}
          <div class="analysis-review-player">
            <div>
              <strong>Reviewing {kindLabel(reviewedCandidate.kind)}</strong>
              <span>{formatTimelineTime(reviewedCandidate.eventMs)}</span>
            </div>
            <!-- svelte-ignore a11y_media_has_caption -->
            <video
              bind:this={reviewVideo}
              src={sourceUrl}
              aria-label={`Source preview at ${formatTimelineTime(reviewedCandidate.eventMs)}`}
              controls
              preload="metadata"
              onloadedmetadata={seekReviewVideo}
            ></video>
          </div>
        {/if}

        <div class="analysis-candidate-list">
          {#each analysisState.candidates as candidate (candidate.id)}
            {@const windowedCandidate = configuredCandidate(candidate)}
            <article class={`analysis-candidate analysis-${candidate.kind}`}>
              <div class="analysis-candidate-heading">
                <div>
                  <span>{kindLabel(candidate.kind)}</span>
                  <strong>{formatTimelineTime(candidate.eventMs)}</strong>
                </div>
                <small>{confidenceLabel(candidate.confidence)}</small>
              </div>
              <p>{candidate.evidence.join(" · ")}</p>
              <dl>
                <div>
                  <dt>Suggested range</dt>
                  <dd>
                    {formatTimelineTime(windowedCandidate.suggestedInMs)} →
                    {formatTimelineTime(windowedCandidate.suggestedOutMs)}
                  </dd>
                </div>
              </dl>
              <div class="analysis-candidate-actions">
                <button
                  class="secondary-button"
                  type="button"
                  onclick={() => review(candidate)}
                >
                  Review moment
                </button>
                <button
                  class="primary-button"
                  type="button"
                  onclick={() => onApply(windowedCandidate)}
                  disabled={extractionWindowError !== null}
                >
                  Use suggested range
                </button>
              </div>
            </article>
          {/each}
        </div>
      </section>
    {:else}
      <section class="analysis-message" role="status">
        <strong>Ready to inspect the source.</strong>
        <p>
          Clip Forge samples small frames through FFmpeg. The original media
          remains on this device.
        </p>
      </section>
    {/if}

    <footer class="analysis-footer">
      <span>Local frame analysis · no upload</span>
      <span>Diablo IV profile · heuristic suggestions</span>
    </footer>
  </div>
</div>
