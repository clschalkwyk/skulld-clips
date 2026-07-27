<script lang="ts">
  import type {
    ExportSettings,
    ExportValidation,
  } from "../../../contracts/types";
  import type { ExportState } from "../../services/export-state";

  interface Props {
    projectName: string;
    settings: ExportSettings;
    sourceHasAudio: boolean;
    destinationPath: string;
    overwriteConfirmed: boolean;
    state: ExportState;
    diagnosticBusy: boolean;
    diagnosticPath: string | null;
    diagnosticError: string | null;
    onSettingsChange: (settings: ExportSettings) => void;
    onChooseDestination: () => void;
    onOverwriteChange: (confirmed: boolean) => void;
    onStart: () => void;
    onCancel: () => void;
    onCreateDiagnostic: () => void;
    onReveal: () => void;
    onClose: () => void;
  }

  let {
    projectName,
    settings,
    sourceHasAudio,
    destinationPath,
    overwriteConfirmed,
    state,
    diagnosticBusy,
    diagnosticPath,
    diagnosticError,
    onSettingsChange,
    onChooseDestination,
    onOverwriteChange,
    onStart,
    onCancel,
    onCreateDiagnostic,
    onReveal,
    onClose,
  }: Props = $props();

  const active = $derived(
    state.status === "starting" || state.status === "running",
  );
  const outputExists = $derived(
    state.validation?.errors.some(({ code }) => code === "E_OUTPUT_EXISTS") ??
      false,
  );
  const nonOverwriteErrors = $derived(
    state.validation?.errors.filter(({ code }) => code !== "E_OUTPUT_EXISTS") ??
      [],
  );
  const canStart = $derived(
    destinationPath.length > 0 &&
      !active &&
      nonOverwriteErrors.length === 0 &&
      (!outputExists || overwriteConfirmed),
  );

  function updateSetting<Key extends keyof ExportSettings>(
    key: Key,
    value: ExportSettings[Key],
  ): void {
    onSettingsChange({ ...settings, [key]: value });
  }

  function filename(path: string): string {
    return path.split(/[\\/]/).at(-1) ?? path;
  }

  function formatBytes(bytes: number | null): string {
    if (bytes === null) {
      return "Calculated during validation";
    }
    const units = ["B", "KB", "MB", "GB"];
    let value = bytes;
    let unit = 0;
    while (value >= 1024 && unit < units.length - 1) {
      value /= 1024;
      unit += 1;
    }
    return `${value.toFixed(unit === 0 ? 0 : 1)} ${units[unit]}`;
  }

  function phaseLabel(phase: string | null): string {
    switch (phase) {
      case "preparing-assets":
        return "Preparing assets";
      case "encoding":
        return "Encoding video";
      case "verifying":
        return "Verifying output";
      default:
        return state.status === "starting" ? "Starting export" : "Export";
    }
  }

  function validationForDisplay(): ExportValidation | null {
    return state.validation;
  }
</script>

<div class="export-backdrop" role="presentation">
  <div
    class="export-sheet"
    role="dialog"
    aria-modal="true"
    aria-labelledby="export-title"
  >
    <header class="export-heading">
      <div>
        <p class="section-label">Verified local export</p>
        <h2 id="export-title">Export vertical MP4</h2>
        <p>{projectName}</p>
      </div>
      <button
        class="text-button"
        type="button"
        onclick={onClose}
        disabled={active}
        aria-label="Close export"
      >Close</button>
    </header>

    {#if state.status === "completed"}
      <div class="export-terminal export-success" role="status">
        <strong>Export verified and saved</strong>
        <p>{state.outputPath ? filename(state.outputPath) : "MP4 complete"}</p>
        <span>{formatBytes(state.outputBytes)}</span>
      </div>
    {:else if state.status === "cancelled"}
      <div class="export-terminal" role="status">
        <strong>Export cancelled cleanly</strong>
        <p>No partial output was kept. The editor is ready for another export.</p>
      </div>
    {:else}
      <div class="export-controls">
        <section class="export-destination">
          <span>Destination</span>
          <strong>
            {destinationPath
              ? filename(destinationPath)
              : "No output selected"}
          </strong>
          <small>
            {destinationPath
              ? "Saved to the folder you approved."
              : "Choose a local MP4 destination to continue."}
          </small>
          <button
            class="secondary-button"
            type="button"
            onclick={onChooseDestination}
            disabled={active}
          >
            {destinationPath ? "Change destination" : "Choose destination"}
          </button>
        </section>

        <div class="export-setting-grid">
          <label>
            Quality
            <select
              value={settings.qualityMode}
              onchange={(event) =>
                updateSetting(
                  "qualityMode",
                  (event.currentTarget as HTMLSelectElement)
                    .value as ExportSettings["qualityMode"],
                )}
              disabled={active}
            >
              <option value="draft">Draft · faster</option>
              <option value="balanced">Balanced</option>
              <option value="high">High · slower</option>
            </select>
          </label>
          <label>
            Frame rate
            <select
              value={settings.frameRateMode}
              onchange={(event) =>
                updateSetting(
                  "frameRateMode",
                  (event.currentTarget as HTMLSelectElement)
                    .value as ExportSettings["frameRateMode"],
                )}
              disabled={active}
            >
              <option value="source-capped-60">Source, capped at 60</option>
              <option value="30">30 fps</option>
              <option value="60">60 fps</option>
            </select>
          </label>
          <label>
            Audio
            <select
              value={settings.audioBitrateKbps}
              onchange={(event) =>
                updateSetting(
                  "audioBitrateKbps",
                  Number(
                    (event.currentTarget as HTMLSelectElement).value,
                  ) as ExportSettings["audioBitrateKbps"],
                )}
              disabled={active || !sourceHasAudio}
            >
              <option value="128">AAC 128 kbps</option>
              <option value="160">AAC 160 kbps</option>
              <option value="192">AAC 192 kbps</option>
              <option value="256">AAC 256 kbps</option>
            </select>
            {#if !sourceHasAudio}<small>Source has no audio stream.</small>{/if}
          </label>
        </div>

        <dl class="export-summary">
          <div><dt>Output</dt><dd>1080 × 1920 MP4</dd></div>
          <div><dt>Video</dt><dd>H.264 · yuv420p</dd></div>
          <div>
            <dt>Estimated size</dt>
            <dd>{formatBytes(validationForDisplay()?.estimatedBytes ?? null)}</dd>
          </div>
          <div>
            <dt>Free space</dt>
            <dd>{formatBytes(validationForDisplay()?.freeBytes ?? null)}</dd>
          </div>
        </dl>

        {#if state.validation && state.validation.warnings.length > 0}
          <div class="export-warnings">
            {#each state.validation.warnings as warning (warning)}
              <p>{warning}</p>
            {/each}
          </div>
        {/if}

        {#if outputExists}
          <label class="overwrite-confirmation">
            <input
              type="checkbox"
              checked={overwriteConfirmed}
              onchange={(event) =>
                onOverwriteChange(
                  (event.currentTarget as HTMLInputElement).checked,
                )}
            />
            Replace the existing file at this destination
          </label>
        {/if}

        {#if nonOverwriteErrors.length > 0}
          <div class="export-errors" role="alert">
            {#each nonOverwriteErrors as error (error.code)}
              <div>
                <strong>{error.message}</strong>
                {#if error.safeDetail}<p>{error.safeDetail}</p>{/if}
              </div>
            {/each}
          </div>
        {/if}

        {#if state.error}
          <div class="export-errors" role="alert">
            <strong>{state.error.message}</strong>
            {#if state.error.safeDetail}<p>{state.error.safeDetail}</p>{/if}
          </div>
        {/if}

        {#if active}
          <div class="export-progress" aria-live="polite">
            <div>
              <strong>{phaseLabel(state.phase)}</strong>
              <span>{Math.round(state.progress * 100)}%</span>
            </div>
            <progress max="1" value={state.progress}></progress>
            <small>
              {state.fps ? `${state.fps.toFixed(1)} fps` : "Working locally"}
              {state.speed ? ` · ${state.speed.toFixed(2)}×` : ""}
            </small>
          </div>
        {/if}
      </div>
    {/if}

    {#if diagnosticPath}
      <div class="diagnostic-result" role="status">
        Diagnostic ZIP created: {filename(diagnosticPath)}
      </div>
    {/if}
    {#if diagnosticError}
      <div class="export-errors" role="alert">{diagnosticError}</div>
    {/if}

    <footer class="export-actions">
      {#if active}
        <button
          class="secondary-button"
          type="button"
          onclick={onCancel}
          disabled={state.cancelRequested}
        >
          {state.cancelRequested ? "Cancelling…" : "Cancel export"}
        </button>
      {:else if state.status === "completed" || state.status === "cancelled"}
        {#if state.status === "completed"}
          <button class="secondary-button" type="button" onclick={onReveal}>
            Reveal output
          </button>
        {/if}
        <button class="primary-button" type="button" onclick={onClose}>Done</button>
      {:else}
        {#if state.status === "error"}
          <button
            class="secondary-button"
            type="button"
            onclick={onCreateDiagnostic}
            disabled={diagnosticBusy}
          >
            {diagnosticBusy ? "Creating ZIP…" : "Create diagnostic ZIP"}
          </button>
        {/if}
        <button class="text-button" type="button" onclick={onClose}>Not now</button>
        <button
          class="primary-button"
          type="button"
          onclick={onStart}
          disabled={!canStart || state.status === "validating"}
        >
          {state.status === "validating" ? "Checking export…" : "Export video"}
        </button>
      {/if}
    </footer>
  </div>
</div>
