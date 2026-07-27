<script lang="ts">
  import type { Overlay } from "../../../contracts/types";
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
    disabled?: boolean;
    onInChange: (milliseconds: number) => void;
    onOutChange: (milliseconds: number) => void;
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
    disabled = false,
    onInChange,
    onOutChange,
    onPlayheadChange,
    onPlayingChange,
  }: Props = $props();

  const selectionStyle = $derived(
    `left:${(inMs / durationMs) * 100}%;width:${((outMs - inMs) / durationMs) * 100}%`,
  );

  function inputNumber(event: Event): number {
    return Number((event.currentTarget as HTMLInputElement).value);
  }

  function overlayBarStyle(overlay: Overlay): string {
    return `left:${(overlay.startMs / durationMs) * 100}%;width:${((overlay.endMs - overlay.startMs) / durationMs) * 100}%`;
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
    <span class="range-duration">
      Range {formatTimelineTime(outMs - inMs)}
    </span>
  </div>

  <div class="timeline-track">
    <div class="selection-range" style={selectionStyle}></div>
    <div class="overlay-bars" aria-label="Overlay visibility ranges">
      {#each overlays as overlay (overlay.id)}
        <span
          style={overlayBarStyle(overlay)}
          title={`${overlay.name} visibility`}
        ></span>
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
