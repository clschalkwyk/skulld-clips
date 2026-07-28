<script lang="ts">
  import type {
    CaptionStyle,
    NormalizedRect,
    Overlay,
  } from "../../../contracts/types";
  import {
    overlayAsset,
    resetOverlayPosition,
    resetStingPosition,
    resizeOverlay,
  } from "../../services/overlay-model";

  interface Props {
    overlay: Overlay;
    timelineInMs: number;
    timelineOutMs: number;
    captionStatus?: "idle" | "rendering" | "error";
    disabled?: boolean;
    onChange: (overlay: Overlay) => void;
    onCaptionChange: (id: string, caption: CaptionStyle) => void;
    onReplaceImage: (id: string) => Promise<void>;
    onReplaceSting: (id: string) => Promise<void>;
    onReorder: (id: string, direction: -1 | 1) => void;
    onDelete: (id: string) => void;
  }

  let {
    overlay,
    timelineInMs,
    timelineOutMs,
    captionStatus = "idle",
    disabled = false,
    onChange,
    onCaptionChange,
    onReplaceImage,
    onReplaceSting,
    onReorder,
    onDelete,
  }: Props = $props();

  function numberValue(event: Event): number {
    return Number((event.currentTarget as HTMLInputElement).value);
  }

  function stringValue(event: Event): string {
    return (event.currentTarget as HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement).value;
  }

  function updatePosition(partial: Partial<NormalizedRect>): void {
    const position = { ...overlay.position, ...partial };
    position.x = clamp(position.x, 0, 1 - position.width);
    position.y = clamp(position.y, 0, 1 - position.height);
    onChange({ ...overlay, position });
  }

  function updateWidth(width: number): void {
    onChange({
      ...overlay,
      position: resizeOverlay(overlay.position, overlayAsset(overlay), width),
    });
  }

  function updateStart(value: number): void {
    const minimumDuration = overlay.type === "sting" ? 500 : 1;
    onChange({
      ...overlay,
      startMs: Math.round(
        clamp(value, timelineInMs, overlay.endMs - minimumDuration),
      ),
    });
  }

  function updateEnd(value: number): void {
    const minimumDuration = overlay.type === "sting" ? 500 : 1;
    const maximumEnd =
      overlay.type === "sting"
        ? Math.min(
            timelineOutMs,
            overlay.startMs + Math.floor(overlay.asset.durationMs / 3),
          )
        : timelineOutMs;
    onChange({
      ...overlay,
      endMs: Math.round(
        clamp(value, overlay.startMs + minimumDuration, maximumEnd),
      ),
    });
  }

  function updateCaption(partial: Partial<CaptionStyle>): void {
    if (overlay.type === "caption") {
      onCaptionChange(overlay.id, { ...overlay.caption, ...partial });
    }
  }

  function applyPreset(preset: "bold" | "panel"): void {
    if (preset === "bold") {
      updateCaption({
        fontSizePx: 72,
        fontWeight: 900,
        align: "center",
        lineHeight: 1.05,
        maxWidthPx: 920,
        fill: "#ffffff",
        outlineWidthPx: 6,
        outlineColor: "#000000",
        backgroundEnabled: false,
        paddingPx: 28,
      });
    } else {
      updateCaption({
        fontSizePx: 58,
        fontWeight: 700,
        align: "center",
        lineHeight: 1.15,
        maxWidthPx: 860,
        fill: "#ffffff",
        outlineWidthPx: 0,
        backgroundEnabled: true,
        backgroundColor: "#000000",
        paddingPx: 30,
      });
    }
  }

  function clamp(value: number, minimum: number, maximum: number): number {
    return Math.min(maximum, Math.max(minimum, Number.isFinite(value) ? value : minimum));
  }
</script>

<section class="overlay-inspector" aria-labelledby="overlay-heading">
  <div class="inspector-heading">
    <div>
      <p class="section-label">{overlay.type} overlay</p>
      <h2 id="overlay-heading">{overlay.name}</h2>
    </div>
    <button type="button" disabled={disabled} onclick={() => onDelete(overlay.id)}>
      Delete
    </button>
  </div>

  <label class="field-control">
    <span>Name</span>
    <input
      type="text"
      required
      maxlength="120"
      value={overlay.name}
      disabled={disabled}
      onchange={(event) => {
        const name = stringValue(event).trim();
        if (name) {
          onChange({ ...overlay, name });
        }
      }}
    />
  </label>

  <div class="order-actions">
    <button type="button" disabled={disabled} onclick={() => onReorder(overlay.id, -1)}>
      Send backward
    </button>
    <button type="button" disabled={disabled} onclick={() => onReorder(overlay.id, 1)}>
      Bring forward
    </button>
  </div>

  <label class="field-control">
    <span>Opacity <strong>{Math.round(overlay.opacity * 100)}%</strong></span>
    <input
      type="range"
      min="0"
      max="1"
      step="0.01"
      value={overlay.opacity}
      disabled={disabled}
      oninput={(event) =>
        onChange({
          ...overlay,
          opacity: clamp(numberValue(event), 0, 1),
        })}
    />
  </label>

  <div class="position-inputs overlay-position-inputs">
    <label>
      <span>X</span>
      <input
        type="number"
        min="0"
        max={1 - overlay.position.width}
        step="0.000001"
        value={overlay.position.x}
        disabled={disabled}
        oninput={(event) => updatePosition({ x: numberValue(event) })}
      />
    </label>
    <label>
      <span>Y</span>
      <input
        type="number"
        min="0"
        max={1 - overlay.position.height}
        step="0.000001"
        value={overlay.position.y}
        disabled={disabled}
        oninput={(event) => updatePosition({ y: numberValue(event) })}
      />
    </label>
    <label>
      <span>Width</span>
      <input
        type="number"
        min={32 / 1080}
        max="1"
        step="0.000001"
        value={overlay.position.width}
        disabled={disabled}
        oninput={(event) => updateWidth(numberValue(event))}
      />
    </label>
    <label>
      <span>Height</span>
      <input type="number" value={overlay.position.height} disabled readonly />
    </label>
  </div>

  <div class="timing-inputs">
    <label>
      <span>Start ms</span>
      <input
        type="number"
        min={timelineInMs}
        max={overlay.endMs - (overlay.type === "sting" ? 500 : 1)}
        step="1"
        value={overlay.startMs}
        disabled={disabled}
        oninput={(event) => updateStart(numberValue(event))}
      />
    </label>
    <label>
      <span>End ms</span>
      <input
        type="number"
        min={overlay.startMs + (overlay.type === "sting" ? 500 : 1)}
        max={overlay.type === "sting"
          ? Math.min(
              timelineOutMs,
              overlay.startMs + Math.floor(overlay.asset.durationMs / 3),
            )
          : timelineOutMs}
        step="1"
        value={overlay.endMs}
        disabled={disabled}
        oninput={(event) => updateEnd(numberValue(event))}
      />
    </label>
  </div>

  {#if overlay.type === "image"}
    <div class="asset-actions">
      <button type="button" disabled={disabled} onclick={() => onReplaceImage(overlay.id)}>
        Replace image
      </button>
      <button
        type="button"
        disabled={disabled}
        onclick={() =>
          onChange({
            ...overlay,
            position: resetOverlayPosition(overlay.asset),
          })}
      >
        Reset position
      </button>
    </div>
    <p class="asset-note">
      {overlay.asset.width} × {overlay.asset.height} · {overlay.asset.mimeType}
    </p>
  {:else if overlay.type === "caption"}
    <div class="caption-status" data-status={captionStatus} aria-live="polite">
      {captionStatus === "rendering"
        ? "Rendering caption…"
        : captionStatus === "error"
          ? "Caption render failed"
          : "Caption asset current"}
    </div>
    <label class="field-control">
      <span>Text</span>
      <textarea
        rows="5"
        maxlength="500"
        value={overlay.caption.text}
        disabled={disabled}
        oninput={(event) => updateCaption({ text: stringValue(event) })}
      ></textarea>
    </label>
    <div class="preset-actions">
      <button type="button" disabled={disabled} onclick={() => applyPreset("bold")}>
        Bold hook
      </button>
      <button type="button" disabled={disabled} onclick={() => applyPreset("panel")}>
        Solid panel
      </button>
    </div>
    <div class="caption-style-grid">
      <label>
        <span>Font</span>
        <select disabled>
          <option>Inter</option>
        </select>
      </label>
      <label>
        <span>Weight</span>
        <select
          value={overlay.caption.fontWeight}
          disabled={disabled}
          onchange={(event) => updateCaption({ fontWeight: Number(stringValue(event)) })}
        >
          <option value="400">Regular</option>
          <option value="700">Bold</option>
          <option value="900">Black</option>
        </select>
      </label>
      <label>
        <span>Size px</span>
        <input
          type="number"
          min="12"
          max="300"
          value={overlay.caption.fontSizePx}
          disabled={disabled}
          oninput={(event) =>
            updateCaption({
              fontSizePx: clamp(numberValue(event), 12, 300),
            })}
        />
      </label>
      <label>
        <span>Align</span>
        <select
          value={overlay.caption.align}
          disabled={disabled}
          onchange={(event) =>
            updateCaption({
              align: stringValue(event) as CaptionStyle["align"],
            })}
        >
          <option value="left">Left</option>
          <option value="center">Center</option>
          <option value="right">Right</option>
        </select>
      </label>
      <label>
        <span>Line height</span>
        <input
          type="number"
          min="0.8"
          max="3"
          step="0.05"
          value={overlay.caption.lineHeight}
          disabled={disabled}
          oninput={(event) =>
            updateCaption({
              lineHeight: clamp(numberValue(event), 0.8, 3),
            })}
        />
      </label>
      <label>
        <span>Max width</span>
        <input
          type="number"
          min="50"
          max="1080"
          value={overlay.caption.maxWidthPx}
          disabled={disabled}
          oninput={(event) =>
            updateCaption({
              maxWidthPx: clamp(numberValue(event), 50, 1080),
            })}
        />
      </label>
      <label>
        <span>Fill</span>
        <input
          type="color"
          value={overlay.caption.fill.slice(0, 7)}
          disabled={disabled}
          oninput={(event) => updateCaption({ fill: stringValue(event) })}
        />
      </label>
      <label>
        <span>Outline</span>
        <input
          type="number"
          min="0"
          max="30"
          value={overlay.caption.outlineWidthPx}
          disabled={disabled}
          oninput={(event) =>
            updateCaption({
              outlineWidthPx: clamp(numberValue(event), 0, 30),
            })}
        />
      </label>
      <label>
        <span>Outline color</span>
        <input
          type="color"
          value={overlay.caption.outlineColor.slice(0, 7)}
          disabled={disabled}
          oninput={(event) => updateCaption({ outlineColor: stringValue(event) })}
        />
      </label>
      <label>
        <span>Padding</span>
        <input
          type="number"
          min="0"
          max="100"
          value={overlay.caption.paddingPx}
          disabled={disabled}
          oninput={(event) =>
            updateCaption({
              paddingPx: clamp(numberValue(event), 0, 100),
            })}
        />
      </label>
    </div>
    <label class="checkbox-control">
      <input
        type="checkbox"
        checked={overlay.caption.backgroundEnabled}
        disabled={disabled}
        onchange={(event) =>
          updateCaption({
            backgroundEnabled: (event.currentTarget as HTMLInputElement).checked,
          })}
      />
      <span>Solid background</span>
    </label>
    {#if overlay.caption.backgroundEnabled}
      <label class="field-control">
        <span>Background color</span>
        <input
          type="color"
          value={overlay.caption.backgroundColor.slice(0, 7)}
          disabled={disabled}
          oninput={(event) => updateCaption({ backgroundColor: stringValue(event) })}
        />
      </label>
    {/if}
  {:else}
    <div class="asset-actions">
      <button type="button" disabled={disabled} onclick={() => onReplaceSting(overlay.id)}>
        Replace sting MP4
      </button>
      <button
        type="button"
        disabled={disabled}
        onclick={() =>
          onChange({
            ...overlay,
            position: resetStingPosition(overlay.asset),
          })}
      >
        Reset safe placement
      </button>
    </div>
    <label class="checkbox-control">
      <input
        type="checkbox"
        checked={overlay.includeAudio}
        disabled={disabled || !overlay.asset.hasAudio}
        onchange={(event) =>
          onChange({
            ...overlay,
            includeAudio: (event.currentTarget as HTMLInputElement).checked,
          })}
      />
      <span>Include sting audio</span>
    </label>
    <p class="asset-note">
      Toasty-right · 3× speed · fixed green key<br />
      {overlay.asset.width} × {overlay.asset.height} ·
      {(overlay.asset.durationMs / 1_000).toFixed(2)} seconds ·
      {overlay.asset.hasAudio ? "audio detected" : "silent"}
    </p>
  {/if}
</section>
