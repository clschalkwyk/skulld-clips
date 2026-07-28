import type {
  AssetRef,
  CaptionOverlay,
  CaptionStyle,
  ImageOverlay,
  NormalizedRect,
  Overlay,
  StingAssetRef,
  StingOverlay,
} from "../../contracts/types";

const OUTPUT_WIDTH = 1080;
const OUTPUT_HEIGHT = 1920;
const MIN_OVERLAY_PIXELS = 32;
export const DEFAULT_STING_PLAYBACK_RATE = 1;
export const LEGACY_STING_PLAYBACK_RATE = 3;
export const MAX_STING_OVERLAYS = 8;
export const MAX_STING_REPEAT_DURATION_MS = 60_000;
export const STING_ENTRY_MS = 180;
export const STING_EXIT_MS = 120;
const OUTPUT_SAFE_X_PX = 24;
const OUTPUT_SAFE_Y_PX = 80;

export type OverlayAnchor =
  | "top-left"
  | "top-center"
  | "top-right"
  | "middle-left"
  | "center"
  | "middle-right"
  | "bottom-left"
  | "bottom-center"
  | "bottom-right";

export const DEFAULT_CAPTION_STYLE: CaptionStyle = {
  text: "",
  fontFamily: "Inter",
  fontSizePx: 72,
  fontWeight: 900,
  align: "center",
  lineHeight: 1.05,
  maxWidthPx: 920,
  fill: "#ffffff",
  outlineWidthPx: 6,
  outlineColor: "#000000",
  backgroundEnabled: false,
  backgroundColor: "#000000",
  paddingPx: 28,
};

export function createImageOverlay(
  id: string,
  asset: AssetRef,
  inMs: number,
  outMs: number,
  zIndex: number,
): ImageOverlay {
  return {
    id,
    type: "image",
    name: asset.originalFilename ?? "Brand image",
    asset,
    position: centeredAssetPosition(asset, 0.24),
    opacity: 1,
    startMs: inMs,
    endMs: outMs,
    zIndex,
  };
}

export function createCaptionOverlay(
  id: string,
  text: string,
  style: CaptionStyle,
  asset: AssetRef,
  inMs: number,
  outMs: number,
  zIndex: number,
): CaptionOverlay {
  const position = centeredAssetPosition(asset, Math.min(0.84, asset.width / OUTPUT_WIDTH));
  position.y = clamp(0.12, 0, 1 - position.height);
  return {
    id,
    type: "caption",
    name: "Hook caption",
    caption: { ...style, text },
    generatedAsset: asset,
    position,
    opacity: 1,
    startMs: inMs,
    endMs: outMs,
    zIndex,
  };
}

export function createStingOverlay(
  id: string,
  asset: StingAssetRef,
  inMs: number,
  outMs: number,
  zIndex: number,
): StingOverlay {
  const durationMs = Math.min(
    Math.floor(asset.durationMs / DEFAULT_STING_PLAYBACK_RATE),
    outMs - inMs,
  );
  if (durationMs < 500) {
    throw new Error("The active range must leave at least 500 ms for the Skull'd sting.");
  }
  const startMs = Math.min(inMs + 2_000, outMs - durationMs);
  return {
    id,
    type: "sting",
    name: asset.originalFilename ?? "Skull'd sting",
    asset,
    preset: "toasty-right",
    includeAudio: asset.hasAudio,
    playbackRate: DEFAULT_STING_PLAYBACK_RATE,
    repeat: false,
    position: stingSafePosition(asset),
    opacity: 1,
    startMs,
    endMs: startMs + durationMs,
    zIndex,
  };
}

export function insertStingOverlayAtPlayhead(
  source: StingOverlay,
  id: string,
  playheadMs: number,
  inMs: number,
  outMs: number,
  zIndex: number,
): StingOverlay {
  if (outMs - inMs < 500) {
    throw new Error("The active range must leave at least 500 ms for the Skull'd sting.");
  }
  const startMs = Math.round(clamp(playheadMs, inMs, outMs - 500));
  const durationMs = Math.min(source.endMs - source.startMs, outMs - startMs);
  return {
    ...source,
    id,
    position: { ...source.position },
    startMs,
    endMs: startMs + durationMs,
    zIndex,
  };
}

export function stingPlaybackRate(overlay: StingOverlay): 1 | 2 | 3 {
  return overlay.playbackRate ?? LEGACY_STING_PLAYBACK_RATE;
}

export function stingRepeats(overlay: StingOverlay): boolean {
  return overlay.repeat ?? false;
}

export function stingCycleDurationMs(overlay: StingOverlay): number {
  return Math.floor(overlay.asset.durationMs / stingPlaybackRate(overlay));
}

export function maximumStingDurationMs(
  overlay: StingOverlay,
  timelineOutMs: number,
): number {
  const available = Math.max(500, timelineOutMs - overlay.startMs);
  return stingRepeats(overlay)
    ? Math.min(available, MAX_STING_REPEAT_DURATION_MS)
    : Math.min(available, stingCycleDurationMs(overlay));
}

export function setStingPlaybackRate(
  overlay: StingOverlay,
  playbackRate: 1 | 2 | 3,
  timelineOutMs: number,
): StingOverlay {
  const updated = { ...overlay, playbackRate };
  if (stingRepeats(updated)) {
    return {
      ...updated,
      endMs:
        updated.startMs +
        Math.min(
          updated.endMs - updated.startMs,
          maximumStingDurationMs(updated, timelineOutMs),
        ),
    };
  }
  return {
    ...updated,
    endMs:
      updated.startMs +
      Math.min(
        stingCycleDurationMs(updated),
        timelineOutMs - updated.startMs,
      ),
  };
}

export function setStingRepeat(
  overlay: StingOverlay,
  repeat: boolean,
  timelineOutMs: number,
): StingOverlay {
  const updated = { ...overlay, repeat };
  const cycleDurationMs = stingCycleDurationMs(updated);
  const requestedDurationMs = repeat
    ? Math.max(updated.endMs - updated.startMs, cycleDurationMs * 2)
    : cycleDurationMs;
  return {
    ...updated,
    endMs:
      updated.startMs +
      Math.min(
        requestedDurationMs,
        maximumStingDurationMs(updated, timelineOutMs),
      ),
  };
}

export function setStingDuration(
  overlay: StingOverlay,
  durationMs: number,
  timelineOutMs: number,
): StingOverlay {
  return {
    ...overlay,
    endMs:
      overlay.startMs +
      Math.round(
        clamp(
          durationMs,
          500,
          maximumStingDurationMs(overlay, timelineOutMs),
        ),
      ),
  };
}

export function isOverlayVisible(overlay: Overlay, playheadMs: number): boolean {
  return playheadMs >= overlay.startMs && playheadMs <= overlay.endMs;
}

export function stingDisplayX(overlay: StingOverlay, playheadMs: number): number {
  const entryEndMs = overlay.startMs + STING_ENTRY_MS;
  const exitStartMs = overlay.endMs - STING_EXIT_MS;
  if (playheadMs < entryEndMs) {
    return lerp(
      1,
      overlay.position.x,
      clamp((playheadMs - overlay.startMs) / STING_ENTRY_MS, 0, 1),
    );
  }
  if (playheadMs > exitStartMs) {
    return lerp(
      overlay.position.x,
      1,
      clamp((playheadMs - exitStartMs) / STING_EXIT_MS, 0, 1),
    );
  }
  return overlay.position.x;
}

export function moveOverlay(
  position: NormalizedRect,
  deltaX: number,
  deltaY: number,
): NormalizedRect {
  return roundRect({
    ...position,
    x: clamp(position.x + deltaX, 0, 1 - position.width),
    y: clamp(position.y + deltaY, 0, 1 - position.height),
  });
}

export function nudgeOverlay(
  position: NormalizedRect,
  deltaXPixels: number,
  deltaYPixels: number,
): NormalizedRect {
  return moveOverlay(
    position,
    deltaXPixels / OUTPUT_WIDTH,
    deltaYPixels / OUTPUT_HEIGHT,
  );
}

export function anchorOverlay(
  position: NormalizedRect,
  anchor: OverlayAnchor,
): NormalizedRect {
  const horizontal = anchor.endsWith("left")
    ? "left"
    : anchor.endsWith("right")
      ? "right"
      : "center";
  const vertical = anchor.startsWith("top")
    ? "top"
    : anchor.startsWith("bottom")
      ? "bottom"
      : "middle";
  const marginX = Math.min(OUTPUT_SAFE_X_PX / OUTPUT_WIDTH, (1 - position.width) / 2);
  const marginY = Math.min(OUTPUT_SAFE_Y_PX / OUTPUT_HEIGHT, (1 - position.height) / 2);
  const x =
    horizontal === "left"
      ? marginX
      : horizontal === "right"
        ? 1 - position.width - marginX
        : (1 - position.width) / 2;
  const y =
    vertical === "top"
      ? marginY
      : vertical === "bottom"
        ? 1 - position.height - marginY
        : (1 - position.height) / 2;
  return roundRect({ ...position, x, y });
}

export function resizeOverlay(
  position: NormalizedRect,
  asset: AssetRef,
  requestedWidth: number,
): NormalizedRect {
  const aspect = asset.width / asset.height;
  const minimumWidth = MIN_OVERLAY_PIXELS / OUTPUT_WIDTH;
  const maximumWidthFromHeight =
    (1 - position.y) * (OUTPUT_HEIGHT / OUTPUT_WIDTH) * aspect;
  const width = clamp(
    requestedWidth,
    minimumWidth,
    Math.min(1 - position.x, maximumWidthFromHeight),
  );
  const height = width * (OUTPUT_WIDTH / OUTPUT_HEIGHT) / aspect;
  return roundRect({ ...position, width, height });
}

export function resetOverlayPosition(asset: AssetRef): NormalizedRect {
  return centeredAssetPosition(asset, 0.24);
}

export function resetStingPosition(asset: StingAssetRef): NormalizedRect {
  return stingSafePosition(asset);
}

export function nextZIndex(overlays: Overlay[]): number {
  return overlays.reduce((maximum, overlay) => Math.max(maximum, overlay.zIndex), -1) + 1;
}

export function reorderOverlay(
  overlays: Overlay[],
  id: string,
  direction: -1 | 1,
): Overlay[] {
  const ordered = [...overlays].sort((a, b) => a.zIndex - b.zIndex);
  const index = ordered.findIndex((overlay) => overlay.id === id);
  const swapIndex = index + direction;
  if (index < 0 || swapIndex < 0 || swapIndex >= ordered.length) {
    return ordered.map((overlay, zIndex) => ({ ...overlay, zIndex }));
  }
  const selected = ordered[index]!;
  const adjacent = ordered[swapIndex]!;
  ordered[index] = adjacent;
  ordered[swapIndex] = selected;
  return ordered.map((overlay, zIndex) => ({ ...overlay, zIndex }));
}

export function overlayAsset(overlay: Overlay): AssetRef {
  return overlay.type === "caption" ? overlay.generatedAsset : overlay.asset;
}

export function replaceOverlayAsset(
  overlay: Overlay,
  asset: AssetRef,
): Overlay {
  if (overlay.type === "image") {
    return {
      ...overlay,
      asset,
      position: resizeOverlay(overlay.position, asset, overlay.position.width),
    };
  }
  if (overlay.type !== "caption") {
    return overlay;
  }
  const previousIntrinsicWidth = overlay.generatedAsset.width / OUTPUT_WIDTH;
  const scale =
    previousIntrinsicWidth > 0
      ? overlay.position.width / previousIntrinsicWidth
      : 1;
  const resized = resizeOverlay(
    overlay.position,
    asset,
    (asset.width / OUTPUT_WIDTH) * scale,
  );
  const centerX = overlay.position.x + overlay.position.width / 2;
  const centerY = overlay.position.y + overlay.position.height / 2;
  resized.x = roundSix(clamp(centerX - resized.width / 2, 0, 1 - resized.width));
  resized.y = roundSix(clamp(centerY - resized.height / 2, 0, 1 - resized.height));
  return { ...overlay, generatedAsset: asset, position: resized };
}

export function replaceStingAsset(
  overlay: StingOverlay,
  asset: StingAssetRef,
): StingOverlay {
  const updated = { ...overlay, asset };
  const endMs = Math.min(
    overlay.endMs,
    overlay.startMs + maximumStingDurationMs(updated, Number.POSITIVE_INFINITY),
  );
  return {
    ...updated,
    asset,
    includeAudio: overlay.includeAudio && asset.hasAudio,
    endMs,
    position: resizeOverlay(overlay.position, asset, overlay.position.width),
  };
}

function centeredAssetPosition(asset: AssetRef, requestedWidth: number): NormalizedRect {
  const width = clamp(requestedWidth, MIN_OVERLAY_PIXELS / OUTPUT_WIDTH, 1);
  const height = Math.min(
    1,
    width * (OUTPUT_WIDTH / OUTPUT_HEIGHT) / (asset.width / asset.height),
  );
  const fittedWidth = Math.min(
    width,
    height * (OUTPUT_HEIGHT / OUTPUT_WIDTH) * (asset.width / asset.height),
  );
  return roundRect({
    x: (1 - fittedWidth) / 2,
    y: (1 - height) / 2,
    width: fittedWidth,
    height,
  });
}

function stingSafePosition(asset: StingAssetRef): NormalizedRect {
  const width = Math.min(440 / OUTPUT_WIDTH, 1);
  const height = Math.min(
    1,
    width * (OUTPUT_WIDTH / OUTPUT_HEIGHT) / (asset.width / asset.height),
  );
  return roundRect({
    x: clamp(1 - width - 24 / OUTPUT_WIDTH, 0, 1 - width),
    y: clamp(1 - height - 80 / OUTPUT_HEIGHT, 0, 1 - height),
    width,
    height,
  });
}

function roundRect(rect: NormalizedRect): NormalizedRect {
  return {
    x: roundSix(rect.x),
    y: roundSix(rect.y),
    width: roundSix(rect.width),
    height: roundSix(rect.height),
  };
}

function roundSix(value: number): number {
  return Math.round(value * 1_000_000) / 1_000_000;
}

function clamp(value: number, minimum: number, maximum: number): number {
  return Math.min(maximum, Math.max(minimum, value));
}

function lerp(start: number, end: number, amount: number): number {
  return start + (end - start) * amount;
}
