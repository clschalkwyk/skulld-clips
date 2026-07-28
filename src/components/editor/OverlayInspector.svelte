<script lang="ts">
  import type {
    CaptionStyle,
    NormalizedRect,
    Overlay,
  } from "../../../contracts/types";
  import {
    anchorOverlay,
    maximumStingDurationMs,
    nudgeOverlay,
    overlayAsset,
    resetOverlayPosition,
    resetStingPosition,
    resizeOverlay,
    setStingDuration,
    setStingPlaybackRate,
    setStingRepeat,
    stingCycleDurationMs,
    stingPlaybackRate,
    stingRepeats,
    type OverlayAnchor,
  } from "../../services/overlay-model";
  import {
    formatRelativeSeconds,
    placeOverlayAtStartOffset,
    placeOverlayEndAtPlayhead,
    placeOverlayStartAtPlayhead,
    relativeOverlayTiming,
  } from "../../services/overlay-timing";

  interface Props {
    overlay: Overlay;
    timelineInMs: number;
    timelineOutMs: number;
    playheadMs: number;
    captionStatus?: "idle" | "rendering" | "error";
    disabled?: boolean;
    onChange: (overlay: Overlay) => void;
    onCaptionChange: (id: string, caption: CaptionStyle) => void;
    onReplaceImage: (id: string) => Promise<void>;
    onReplaceSting: (id: string) => Promise<void>;
    onDuplicateSting: (id: string) => void;
    onReorder: (id: string, direction: -1 | 1) => void;
    onDelete: (id: string) => void;
  }

  let {
    overlay,
    timelineInMs,
    timelineOutMs,
    playheadMs,
    captionStatus = "idle",
    disabled = false,
    onChange,
    onCaptionChange,
    onReplaceImage,
    onReplaceSting,
    onDuplicateSting,
    onReorder,
    onDelete,
  }: Props = $props();

  let nudgeStepPx = $state(8);
  const timing = $derived(
    relativeOverlayTiming(
      overlay.startMs,
      overlay.endMs,
      timelineInMs,
      timelineOutMs,
    ),
  );
  const timingBarStyle = $derived(
    `left:${timing.leftPercent}%;width:${timing.widthPercent}%`,
  );
  const playheadPercent = $derived(
    clamp(
      ((playheadMs - timelineInMs) /
        Math.max(1, timelineOutMs - timelineInMs)) *
        100,
      0,
      100,
    ),
  );
  const playheadStyle = $derived(`left:${playheadPercent}%`);
  const stingCycleMs = $derived(
    overlay.type === "sting" ? stingCycleDurationMs(overlay) : 0,
  );
  const stingCycleCount = $derived(
    overlay.type === "sting" && stingCycleMs > 0
      ? timing.durationMs / stingCycleMs
      : 0,
  );

  const anchors: Array<{
    id: OverlayAnchor;
    label: string;
    glyph: string;
  }> = [
    { id: "top-left", label: "Top left", glyph: "↖" },
    { id: "top-center", label: "Top center", glyph: "↑" },
    { id: "top-right", label: "Top right", glyph: "↗" },
    { id: "middle-left", label: "Middle left", glyph: "←" },
    { id: "center", label: "Center", glyph: "•" },
    { id: "middle-right", label: "Middle right", glyph: "→" },
    { id: "bottom-left", label: "Bottom left", glyph: "↙" },
    { id: "bottom-center", label: "Bottom center", glyph: "↓" },
    { id: "bottom-right", label: "Bottom right", glyph: "↘" },
  ];

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

  function applyAnchor(anchor: OverlayAnchor): void {
    onChange({
      ...overlay,
      position: anchorOverlay(overlay.position, anchor),
    });
  }

  function nudge(deltaX: number, deltaY: number): void {
    onChange({
      ...overlay,
      position: nudgeOverlay(
        overlay.position,
        deltaX * nudgeStepPx,
        deltaY * nudgeStepPx,
      ),
    });
  }

  function resetPosition(): void {
    onChange({
      ...overlay,
      position:
        overlay.type === "sting"
          ? resetStingPosition(overlay.asset)
          : resetOverlayPosition(overlayAsset(overlay)),
    });
  }

  function isAtAnchor(anchor: OverlayAnchor): boolean {
    const anchored = anchorOverlay(overlay.position, anchor);
    return (
      Math.abs(anchored.x - overlay.position.x) < 0.000001 &&
      Math.abs(anchored.y - overlay.position.y) < 0.000001
    );
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
        ? overlay.startMs + maximumStingDurationMs(overlay, timelineOutMs)
        : timelineOutMs;
    onChange({
      ...overlay,
      endMs: Math.round(
        clamp(value, overlay.startMs + minimumDuration, maximumEnd),
      ),
    });
  }

  function updateStartOffsetSeconds(value: number): void {
    const range = placeOverlayAtStartOffset(
      Math.round(value * 1_000),
      overlay.startMs,
      overlay.endMs,
      timelineInMs,
      timelineOutMs,
    );
    onChange({ ...overlay, ...range });
  }

  function showFullStingAnimation(): void {
    if (overlay.type === "sting") {
      onChange(setStingRepeat(overlay, false, timelineOutMs));
    }
  }

  function showTwoStingLoops(): void {
    if (overlay.type === "sting") {
      const repeating = { ...overlay, repeat: true };
      onChange(
        setStingDuration(
          repeating,
          stingCycleDurationMs(repeating) * 2,
          timelineOutMs,
        ),
      );
    }
  }

  function fillStingRemainder(): void {
    if (overlay.type === "sting") {
      const repeating = { ...overlay, repeat: true };
      onChange(
        setStingDuration(
          repeating,
          timelineOutMs - repeating.startMs,
          timelineOutMs,
        ),
      );
    }
  }

  function updateStingRepeatMode(repeat: boolean): void {
    if (overlay.type === "sting" && stingRepeats(overlay) !== repeat) {
      onChange(setStingRepeat(overlay, repeat, timelineOutMs));
    }
  }

  function setStartAtPlayhead(): void {
    const minimumDurationMs = overlay.type === "sting" ? 500 : 1;
    const maximumDurationMs =
      overlay.type === "sting"
        ? maximumStingDurationMs(overlay, timelineOutMs)
        : Number.POSITIVE_INFINITY;
    const range = placeOverlayStartAtPlayhead(
      playheadMs,
      overlay.startMs,
      overlay.endMs,
      timelineInMs,
      timelineOutMs,
      minimumDurationMs,
      maximumDurationMs,
    );
    onChange({ ...overlay, ...range });
  }

  function setEndAtPlayhead(): void {
    const minimumDurationMs = overlay.type === "sting" ? 500 : 1;
    const maximumDurationMs =
      overlay.type === "sting"
        ? maximumStingDurationMs(overlay, timelineOutMs)
        : Number.POSITIVE_INFINITY;
    const range = placeOverlayEndAtPlayhead(
      playheadMs,
      overlay.startMs,
      overlay.endMs,
      timelineInMs,
      timelineOutMs,
      minimumDurationMs,
      maximumDurationMs,
    );
    onChange({ ...overlay, ...range });
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
      <p class="section-label">{overlay.type}</p>
      <h2 id="overlay-heading" title={overlay.name}>{overlay.name}</h2>
    </div>
    <button type="button" disabled={disabled} onclick={() => onDelete(overlay.id)}>
      Remove
    </button>
  </div>

  <div class="control-section placement-editor">
    <div class="control-section-heading">
      <div>
        <strong>Placement</strong>
        <small>Drag on the preview or choose a safe anchor.</small>
      </div>
      <button type="button" disabled={disabled} onclick={resetPosition}>
        {overlay.type === "sting" ? "Safe corner" : "Reset"}
      </button>
    </div>

    <div class="placement-primary">
      <div class="placement-workbench">
        <div class="placement-tool">
          <span class="control-label">Anchor position</span>
          <div class="anchor-grid anchor-grid-primary" aria-label="Overlay anchor positions">
            {#each anchors as anchor (anchor.id)}
              <button
                type="button"
                class:active={isAtAnchor(anchor.id)}
                aria-label={`Place overlay ${anchor.label.toLowerCase()}`}
                aria-pressed={isAtAnchor(anchor.id)}
                disabled={disabled}
                title={anchor.label}
                onclick={() => applyAnchor(anchor.id)}
              >
                {anchor.glyph}
              </button>
            {/each}
          </div>
        </div>
      </div>

      <div class="placement-sliders primary-size-control">
        <label>
          <span>
            Size
            <strong>{Math.round(overlay.position.width * 1080)} px</strong>
          </span>
          <input
            type="range"
            min={32 / 1080}
            max="1"
            step={1 / 1080}
            value={overlay.position.width}
            disabled={disabled}
            oninput={(event) => updateWidth(numberValue(event))}
          />
        </label>
      </div>
    </div>
  </div>

  <div class="control-section timing-editor">
    <div class="control-section-heading">
      <div>
        <strong>Timing</strong>
        <small>Relative to the selected clip.</small>
      </div>
    </div>
    <div class="relative-timing-summary">
      <div>
        <div class="timing-field-heading">
          <span>Start after in</span>
          <button type="button" disabled={disabled} onclick={setStartAtPlayhead}>
            Playhead
          </button>
        </div>
        {#if overlay.type === "sting"}
          <label class="timing-number-control">
            <input
              type="number"
              min="0"
              max={(timelineOutMs - timelineInMs - timing.durationMs) / 1_000}
              step="0.1"
              value={(timing.startOffsetMs / 1_000).toFixed(2)}
              disabled={disabled}
              aria-label="Sting start in seconds after clip in"
              onchange={(event) => updateStartOffsetSeconds(numberValue(event))}
            />
            <span>s</span>
          </label>
        {:else}
          <strong>
            {timing.startOffsetMs <= 0
              ? "At clip in"
              : `+${formatRelativeSeconds(timing.startOffsetMs)} after in`}
          </strong>
        {/if}
      </div>
      <div>
        <div class="timing-field-heading">
          <span>Duration</span>
          <button type="button" disabled={disabled} onclick={setEndAtPlayhead}>
            End here
          </button>
        </div>
        {#if overlay.type === "sting"}
          <label class="timing-number-control">
            <input
              type="number"
              min="0.5"
              max={maximumStingDurationMs(overlay, timelineOutMs) / 1_000}
              step="0.1"
              value={(timing.durationMs / 1_000).toFixed(2)}
              disabled={disabled}
              aria-label="Sting duration in seconds"
              onchange={(event) =>
                onChange(
                  setStingDuration(
                    overlay,
                    numberValue(event) * 1_000,
                    timelineOutMs,
                  ),
                )}
            />
            <span>s</span>
          </label>
        {:else}
          <strong>{formatRelativeSeconds(timing.durationMs)}</strong>
        {/if}
      </div>
    </div>
    {#if overlay.type === "sting"}
      <div class="sting-playback-row">
        <fieldset>
          <legend>Speed</legend>
          <div class="sting-segmented-control sting-speed-control">
            {#each [1, 2, 3] as playbackRate (playbackRate)}
              <button
                type="button"
                class:active={stingPlaybackRate(overlay) === playbackRate}
                aria-pressed={stingPlaybackRate(overlay) === playbackRate}
                disabled={disabled}
                onclick={() =>
                  onChange(
                    setStingPlaybackRate(
                      overlay,
                      playbackRate as 1 | 2 | 3,
                      timelineOutMs,
                    ),
                  )}
              >
                {playbackRate}×
              </button>
            {/each}
          </div>
        </fieldset>
        <fieldset>
          <legend>Playback</legend>
          <div class="sting-segmented-control">
            <button
              type="button"
              class:active={!stingRepeats(overlay)}
              aria-pressed={!stingRepeats(overlay)}
              disabled={disabled}
              onclick={() => updateStingRepeatMode(false)}
            >
              Once
            </button>
            <button
              type="button"
              class:active={stingRepeats(overlay)}
              aria-pressed={stingRepeats(overlay)}
              disabled={disabled}
              onclick={() => updateStingRepeatMode(true)}
            >
              Repeat
            </button>
          </div>
        </fieldset>
      </div>
      <div class="relative-timing-track" aria-label="Overlay position within selected clip">
        <span class="timing-range" style={timingBarStyle}></span>
        <i class="timing-playhead" style={playheadStyle} aria-hidden="true"></i>
      </div>
      <div class="sting-timing-context" aria-live="polite">
        <span>
          One animation is {formatRelativeSeconds(stingCycleMs)} at
          {stingPlaybackRate(overlay)}×.
        </span>
        <strong>{stingCycleCount.toFixed(stingCycleCount < 10 ? 1 : 0)} cycles</strong>
      </div>
      <div class="sting-timing-presets" aria-label="Sting timing presets">
        <button type="button" disabled={disabled} onclick={showFullStingAnimation}>
          Full animation
        </button>
        <button type="button" disabled={disabled} onclick={showTwoStingLoops}>
          2 loops
        </button>
        <button type="button" disabled={disabled} onclick={fillStingRemainder}>
          Fill remaining
        </button>
      </div>
    {/if}
    {#if overlay.type !== "sting"}
      <div class="relative-timing-track" aria-label="Overlay position within selected clip">
        <span class="timing-range" style={timingBarStyle}></span>
        <i class="timing-playhead" style={playheadStyle} aria-hidden="true"></i>
      </div>
    {/if}
  </div>

  <details class="advanced-overlay">
    <summary>Advanced controls</summary>
    <div class="advanced-overlay-content">
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

      <div class="order-actions">
        <button type="button" disabled={disabled} onclick={() => onReorder(overlay.id, -1)}>
          Send backward
        </button>
        <button type="button" disabled={disabled} onclick={() => onReorder(overlay.id, 1)}>
          Bring forward
        </button>
      </div>

      <div class="advanced-nudge">
        <label class="nudge-step">
          <span class="control-label">Nudge distance</span>
          <select
            aria-label="Overlay nudge distance"
            value={nudgeStepPx}
            disabled={disabled}
            onchange={(event) => (nudgeStepPx = numberValue(event))}
          >
            <option value={1}>1 px</option>
            <option value={8}>8 px</option>
            <option value={24}>24 px</option>
          </select>
        </label>
        <div class="nudge-grid overlay-nudge-grid" aria-label="Overlay nudge controls">
          <button
            type="button"
            aria-label={`Nudge overlay up ${nudgeStepPx} pixels`}
            disabled={disabled}
            onclick={() => nudge(0, -1)}
          >↑</button>
          <button
            type="button"
            aria-label={`Nudge overlay left ${nudgeStepPx} pixels`}
            disabled={disabled}
            onclick={() => nudge(-1, 0)}
          >←</button>
          <button
            type="button"
            aria-label={`Nudge overlay right ${nudgeStepPx} pixels`}
            disabled={disabled}
            onclick={() => nudge(1, 0)}
          >→</button>
          <button
            type="button"
            aria-label={`Nudge overlay down ${nudgeStepPx} pixels`}
            disabled={disabled}
            onclick={() => nudge(0, 1)}
          >↓</button>
        </div>
      </div>

      <div class="placement-sliders">
        <label>
          <span>Left <strong>{Math.round(overlay.position.x * 100)}%</strong></span>
          <input
            type="range"
            min="0"
            max={1 - overlay.position.width}
            step={1 / 1080}
            value={overlay.position.x}
            disabled={disabled}
            oninput={(event) => updatePosition({ x: numberValue(event) })}
          />
        </label>
        <label>
          <span>Top <strong>{Math.round(overlay.position.y * 100)}%</strong></span>
          <input
            type="range"
            min="0"
            max={1 - overlay.position.height}
            step={1 / 1920}
            value={overlay.position.y}
            disabled={disabled}
            oninput={(event) => updatePosition({ y: numberValue(event) })}
          />
        </label>
      </div>

      <details class="exact-placement">
        <summary>Exact placement</summary>
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
      </details>

      <details class="exact-placement">
        <summary>Exact timing</summary>
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
                ? overlay.startMs + maximumStingDurationMs(overlay, timelineOutMs)
                : timelineOutMs}
              step="1"
              value={overlay.endMs}
              disabled={disabled}
              oninput={(event) => updateEnd(numberValue(event))}
            />
          </label>
        </div>
      </details>
    </div>
  </details>

  {#if overlay.type === "image"}
    <div class="asset-actions">
      <button type="button" disabled={disabled} onclick={() => onReplaceImage(overlay.id)}>
        Replace image
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
      <button type="button" disabled={disabled} onclick={() => onDuplicateSting(overlay.id)}>
        Duplicate sting
      </button>
      <button type="button" disabled={disabled} onclick={() => onReplaceSting(overlay.id)}>
        Replace sting MP4
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
      Toasty-right · {stingPlaybackRate(overlay)}× ·
      {stingRepeats(overlay) ? "repeat" : "once"} · fixed green key<br />
      {overlay.asset.width} × {overlay.asset.height} ·
      {(overlay.asset.durationMs / 1_000).toFixed(2)} seconds ·
      {overlay.asset.hasAudio ? "audio detected" : "silent"}
    </p>
  {/if}
</section>
