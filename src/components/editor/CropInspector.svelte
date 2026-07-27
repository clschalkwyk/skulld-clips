<script lang="ts">
  import type { NormalizedRect } from "../../../contracts/types";
  import type { Size } from "../../services/coordinate-mapper";
  import {
    cropZoom,
    moveCrop,
    nudgeCrop,
    resetCrop,
    setCropZoom,
  } from "../../services/crop-solver";

  interface Props {
    crop: NormalizedRect;
    sourceSize: Size;
    disabled?: boolean;
    onChange: (crop: NormalizedRect) => void;
  }

  let {
    crop,
    sourceSize,
    disabled = false,
    onChange,
  }: Props = $props();

  const zoom = $derived(cropZoom(crop, sourceSize));

  function valueFrom(event: Event): number {
    return Number((event.currentTarget as HTMLInputElement).value);
  }

  function setX(value: number): void {
    onChange(moveCrop(crop, { x: value - crop.x, y: 0 }));
  }

  function setY(value: number): void {
    onChange(moveCrop(crop, { x: 0, y: value - crop.y }));
  }
</script>

<section class="crop-inspector" aria-labelledby="crop-heading">
  <div class="inspector-heading">
    <div>
      <p class="section-label">Framing</p>
      <h2 id="crop-heading">Locked 9:16 crop</h2>
    </div>
    <button type="button" disabled={disabled} onclick={() => onChange(resetCrop(sourceSize))}>
      Reset
    </button>
  </div>

  <label class="zoom-control">
    <span>Zoom <strong>{zoom.toFixed(2)}×</strong></span>
    <input
      type="range"
      min="1"
      max="16"
      step="0.01"
      value={zoom}
      disabled={disabled}
      oninput={(event) => onChange(setCropZoom(crop, valueFrom(event), sourceSize))}
    />
  </label>

  <div class="position-inputs">
    <label>
      <span>X</span>
      <input
        type="number"
        min="0"
        max={1 - crop.width}
        step="0.000001"
        value={crop.x}
        disabled={disabled}
        oninput={(event) => setX(valueFrom(event))}
      />
    </label>
    <label>
      <span>Y</span>
      <input
        type="number"
        min="0"
        max={1 - crop.height}
        step="0.000001"
        value={crop.y}
        disabled={disabled}
        oninput={(event) => setY(valueFrom(event))}
      />
    </label>
  </div>

  <div class="nudge-grid" aria-label="Crop nudge controls">
    <button
      type="button"
      aria-label="Nudge crop up"
      disabled={disabled}
      onclick={() => onChange(nudgeCrop(crop, sourceSize, 0, -1))}
    >↑</button>
    <button
      type="button"
      aria-label="Nudge crop left"
      disabled={disabled}
      onclick={() => onChange(nudgeCrop(crop, sourceSize, -1, 0))}
    >←</button>
    <button
      type="button"
      aria-label="Nudge crop right"
      disabled={disabled}
      onclick={() => onChange(nudgeCrop(crop, sourceSize, 1, 0))}
    >→</button>
    <button
      type="button"
      aria-label="Nudge crop down"
      disabled={disabled}
      onclick={() => onChange(nudgeCrop(crop, sourceSize, 0, 1))}
    >↓</button>
  </div>

  <p class="interaction-note">
    Drag the frame to pan. Wheel or the corner handle zooms. Arrow keys nudge;
    Shift moves ten pixels.
  </p>
</section>
