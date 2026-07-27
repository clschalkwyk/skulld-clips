import type { NormalizedRect } from "../../contracts/types";
import type { Point, Size } from "./coordinate-mapper";

const OUTPUT_ASPECT = 9 / 16;
const MINIMUM_CROP_WIDTH_PX = 64;

export function centeredMaximumCrop(source: Size): NormalizedRect {
  const sourceAspect = source.width / source.height;
  if (sourceAspect >= OUTPUT_ASPECT) {
    const width = OUTPUT_ASPECT / sourceAspect;
    return roundRect({
      x: (1 - width) / 2,
      y: 0,
      width,
      height: 1,
    });
  }
  const height = sourceAspect / OUTPUT_ASPECT;
  return roundRect({
    x: 0,
    y: (1 - height) / 2,
    width: 1,
    height,
  });
}

export function moveCrop(
  crop: NormalizedRect,
  delta: Point,
): NormalizedRect {
  return roundRect({
    ...crop,
    x: clamp(crop.x + delta.x, 0, 1 - crop.width),
    y: clamp(crop.y + delta.y, 0, 1 - crop.height),
  });
}

export function zoomCrop(
  crop: NormalizedRect,
  factor: number,
  anchor: Point,
  source: Size,
): NormalizedRect {
  const maximum = centeredMaximumCrop(source);
  const minimumHeight = Math.min(
    maximum.height,
    MINIMUM_CROP_WIDTH_PX / OUTPUT_ASPECT / source.height,
  );
  const height = clamp(crop.height * factor, minimumHeight, maximum.height);
  const width = height * OUTPUT_ASPECT / (source.width / source.height);
  const relativeX = crop.width === 0 ? 0.5 : (anchor.x - crop.x) / crop.width;
  const relativeY = crop.height === 0 ? 0.5 : (anchor.y - crop.y) / crop.height;
  return roundRect({
    x: clamp(anchor.x - relativeX * width, 0, 1 - width),
    y: clamp(anchor.y - relativeY * height, 0, 1 - height),
    width,
    height,
  });
}

export function cropZoom(crop: NormalizedRect, source: Size): number {
  return roundSix(centeredMaximumCrop(source).height / crop.height);
}

export function setCropZoom(
  crop: NormalizedRect,
  zoom: number,
  source: Size,
): NormalizedRect {
  const currentZoom = cropZoom(crop, source);
  return zoomCrop(
    crop,
    currentZoom / clamp(zoom, 1, 16),
    cropCenter(crop),
    source,
  );
}

export function nudgeCrop(
  crop: NormalizedRect,
  source: Size,
  xPixels: number,
  yPixels: number,
): NormalizedRect {
  return moveCrop(crop, {
    x: xPixels / source.width,
    y: yPixels / source.height,
  });
}

export function resetCrop(source: Size): NormalizedRect {
  return centeredMaximumCrop(source);
}

export function cropCenter(crop: NormalizedRect): Point {
  return {
    x: crop.x + crop.width / 2,
    y: crop.y + crop.height / 2,
  };
}

export function isValidLockedCrop(
  crop: NormalizedRect,
  source: Size,
): boolean {
  const pixelAspect =
    crop.width * source.width / (crop.height * source.height);
  return (
    [crop.x, crop.y, crop.width, crop.height].every(Number.isFinite) &&
    crop.x >= 0 &&
    crop.y >= 0 &&
    crop.width > 0 &&
    crop.height > 0 &&
    crop.x + crop.width <= 1.000001 &&
    crop.y + crop.height <= 1.000001 &&
    Math.abs(pixelAspect - OUTPUT_ASPECT) <= 0.002
  );
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
