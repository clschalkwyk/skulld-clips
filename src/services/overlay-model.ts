import type {
  AssetRef,
  CaptionOverlay,
  CaptionStyle,
  ImageOverlay,
  NormalizedRect,
  Overlay,
} from "../../contracts/types";

const OUTPUT_WIDTH = 1080;
const OUTPUT_HEIGHT = 1920;
const MIN_OVERLAY_PIXELS = 32;

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

export function isOverlayVisible(overlay: Overlay, playheadMs: number): boolean {
  return playheadMs >= overlay.startMs && playheadMs <= overlay.endMs;
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
  return overlay.type === "image" ? overlay.asset : overlay.generatedAsset;
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
