<script lang="ts">
  import type { Overlay, StingOverlay } from "../../../contracts/types";
  import {
    findStingStartingAtPlayhead,
    MAX_STING_OVERLAYS,
  } from "../../services/overlay-model";
  import {
    formatTimelineTime,
    setTrimIn,
    setTrimOut,
  } from "../../services/timeline";

  interface Props {
    durationMs: number;
    inMs: number;
    outMs: number;
    playheadMs: number;
    playing: boolean;
    overlays?: Overlay[];
    selectedOverlayId?: string | null;
    disabled?: boolean;
    insertStingDisabled?: boolean;
    onInChange: (milliseconds: number) => void;
    onOutChange: (milliseconds: number) => void;
    onInsertSting: () => Promise<void>;
    onMoveSelectedSting: () => void;
    onOverlaySelect: (id: string) => void;
    onPlayheadChange: (milliseconds: number) => void;
    onPlayingChange: (playing: boolean) => void;
  }

  let {
    durationMs,
    inMs,
    outMs,
    playheadMs,
    playing,
    overlays = [],
    selectedOverlayId = null,
    disabled = false,
    insertStingDisabled = false,
    onInChange,
    onOutChange,
    onInsertSting,
    onMoveSelectedSting,
    onOverlaySelect,
    onPlayheadChange,
    onPlayingChange,
  }: Props = $props();

  const activeDurationMs = $derived(Math.max(1, outMs - inMs));
  const stingOverlays = $derived(
    overlays
      .filter((overlay): overlay is StingOverlay => overlay.type === "sting")
      .sort((a, b) => a.startMs - b.startMs),
  );
  const selectedSting = $derived(
    stingOverlays.find((overlay) => overlay.id === selectedOverlayId) ?? null,
  );
  const activeStings = $derived(
    stingOverlays.filter(
      (overlay) => playheadMs >= overlay.startMs && playheadMs <= overlay.endMs,
    ),
  );
  const activeStingCount = $derived(activeStings.length);
  const stingAtInsertionPoint = $derived(
    findStingStartingAtPlayhead(stingOverlays, playheadMs, inMs, outMs),
  );
  const insertionConflict = $derived(
    stingAtInsertionPoint ?? activeStings[0] ?? null,
  );
  const moveConflict = $derived(
    activeStings.find((sting) => sting.id !== selectedSting?.id) ?? null,
  );

  function inputNumber(event: Event): number {
    return Number((event.currentTarget as HTMLInputElement).value);
  }

  function overlayBarStyle(overlay: Overlay): string {
    const stingLane =
      overlay.type === "sting"
        ? stingOverlays.findIndex((sting) => sting.id === overlay.id)
        : stingOverlays.length;
    return [
      `left:${((overlay.startMs - inMs) / activeDurationMs) * 100}%`,
      `width:${((overlay.endMs - overlay.startMs) / activeDurationMs) * 100}%`,
      `top:${2 + Math.max(0, stingLane) * 4}px`,
    ].join(";");
  }

  function overlayLabel(overlay: Overlay): string {
    if (overlay.type !== "sting") {
      return overlay.name;
    }
    const index = stingOverlays.findIndex((sting) => sting.id === overlay.id);
    return `Sting ${index + 1}`;
  }
</script>

<section class="timeline-panel" aria-label="Clip timeline">
  <div class="transport">
    <button
      class="transport-button"
      type="button"
      disabled={disabled}
      onclick={() => onPlayingChange(!playing)}
      aria-label={playing ? "Pause preview" : "Play preview"}
    >
      {playing ? "Pause" : "Play"}
    </button>
    <span class="time-readout">{formatTimelineTime(playheadMs)}</span>
    <button type="button" disabled={disabled} onclick={() => onInChange(playheadMs)}>
      Set in <kbd>I</kbd>
    </button>
    <button type="button" disabled={disabled} onclick={() => onOutChange(playheadMs)}>
      Set out <kbd>O</kbd>
    </button>
    {#if stingOverlays.length > 0}
      <button
        class="move-sting-button"
        type="button"
        disabled={disabled || !selectedSting || moveConflict !== null}
        aria-label={selectedSting
          ? moveConflict
            ? `${overlayLabel(moveConflict)} is already active at the current playhead`
            : `Move ${overlayLabel(selectedSting)} to the current playhead`
          : "Select a Sting before moving it to the current playhead"}
        title={selectedSting
          ? moveConflict
            ? `${overlayLabel(moveConflict)} is already active here. Move outside its range or adjust its timing first.`
            : `Move ${overlayLabel(selectedSting)} without creating another instance`
          : "Select a Sting from the layer rail or timeline first"}
        onclick={onMoveSelectedSting}
      >
        Move Sting Here
      </button>
    {/if}
    <button
      class="insert-sting-button"
      type="button"
      disabled={disabled || insertStingDisabled || insertionConflict !== null}
      aria-label={insertionConflict
        ? `${overlayLabel(insertionConflict)} is already active at the current playhead`
        : stingOverlays.length === 0
          ? "Add a Skull’d sting at the current playhead"
          : "Insert another Skull’d sting at the current playhead"}
      title={insertionConflict
        ? `${overlayLabel(insertionConflict)} is already active here. Select its timeline bar to edit it.`
        : stingOverlays.length === 0
          ? "Choose a Sting MP4 and place it at the current playhead"
          : "Creates another Sting instance using the selected or latest settings"}
      onclick={onInsertSting}
    >
      {insertionConflict
        ? "Sting Active Here"
        : stingOverlays.length === 0
          ? "Add Sting Here"
          : "Insert Another Sting"}
    </button>
    {#if stingOverlays.length > 0}
      <span class:warning={activeStingCount > 1} class="sting-utility-status">
        {stingOverlays.length}/{MAX_STING_OVERLAYS}
        {stingOverlays.length === 1 ? "Sting" : "Stings"} ·
        {activeStingCount} active here
      </span>
    {/if}
    <span class="range-duration">
      Range {formatTimelineTime(outMs - inMs)}
    </span>
  </div>

  <div class="timeline-track">
    <div class="selection-range"></div>
    <div class="overlay-bars" aria-label="Overlay visibility ranges">
      {#each overlays as overlay (overlay.id)}
        <button
          type="button"
          class="overlay-bar"
          class:sting={overlay.type === "sting"}
          class:selected={selectedOverlayId === overlay.id}
          style={overlayBarStyle(overlay)}
          disabled={disabled}
          aria-label={`Select ${overlayLabel(overlay)} visible from ${formatTimelineTime(overlay.startMs)} to ${formatTimelineTime(overlay.endMs)}`}
          aria-pressed={selectedOverlayId === overlay.id}
          title={`${overlayLabel(overlay)} · ${formatTimelineTime(overlay.startMs)}–${formatTimelineTime(overlay.endMs)}`}
          onclick={() => onOverlaySelect(overlay.id)}
        ></button>
      {/each}
    </div>
    <input
      class="playhead-range"
      type="range"
      aria-label="Playhead"
      min={inMs}
      max={outMs}
      step="1"
      value={playheadMs}
      disabled={disabled}
      oninput={(event) => onPlayheadChange(inputNumber(event))}
    />
  </div>

  <div class="trim-controls">
    <label>
      <span>In</span>
      <input
        type="number"
        min="0"
        max={outMs - 250}
        step="1"
        value={inMs}
        disabled={disabled}
        oninput={(event) =>
          onInChange(setTrimIn(inputNumber(event), outMs, durationMs))}
      />
      <small>{formatTimelineTime(inMs)}</small>
    </label>
    <input
      type="range"
      aria-label="Trim in point"
      min="0"
      max={outMs - 250}
      step="1"
      value={inMs}
      disabled={disabled}
      oninput={(event) =>
        onInChange(setTrimIn(inputNumber(event), outMs, durationMs))}
    />
    <input
      type="range"
      aria-label="Trim out point"
      min={inMs + 250}
      max={durationMs}
      step="1"
      value={outMs}
      disabled={disabled}
      oninput={(event) =>
        onOutChange(setTrimOut(inputNumber(event), inMs, durationMs))}
    />
    <label>
      <span>Out</span>
      <input
        type="number"
        min={inMs + 250}
        max={durationMs}
        step="1"
        value={outMs}
        disabled={disabled}
        oninput={(event) =>
          onOutChange(setTrimOut(inputNumber(event), inMs, durationMs))}
      />
      <small>{formatTimelineTime(outMs)}</small>
    </label>
  </div>
</section>
