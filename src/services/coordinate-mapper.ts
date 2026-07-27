import type { NormalizedRect } from "../../contracts/types";

export interface Size {
  width: number;
  height: number;
}

export interface Point {
  x: number;
  y: number;
}

export interface PixelRect extends Point, Size {}

export function fitSourceInsideStage(source: Size, stage: Size): PixelRect {
  assertPositiveSize(source);
  assertPositiveSize(stage);
  const scale = Math.min(stage.width / source.width, stage.height / source.height);
  const width = roundSix(source.width * scale);
  const height = roundSix(source.height * scale);
  return {
    x: roundSix((stage.width - width) / 2),
    y: roundSix((stage.height - height) / 2),
    width,
    height,
  };
}

export function normalizedRectToStage(
  rect: NormalizedRect,
  source: Size,
  stage: Size,
): PixelRect {
  const fitted = fitSourceInsideStage(source, stage);
  return {
    x: fitted.x + rect.x * fitted.width,
    y: fitted.y + rect.y * fitted.height,
    width: rect.width * fitted.width,
    height: rect.height * fitted.height,
  };
}

export function stagePointToNormalized(
  point: Point,
  source: Size,
  stage: Size,
): Point {
  const fitted = fitSourceInsideStage(source, stage);
  return {
    x: clamp((point.x - fitted.x) / fitted.width, 0, 1),
    y: clamp((point.y - fitted.y) / fitted.height, 0, 1),
  };
}

export function normalizedRectToSourcePixels(
  rect: NormalizedRect,
  source: Size,
): PixelRect {
  assertPositiveSize(source);
  return {
    x: Math.round(rect.x * source.width),
    y: Math.round(rect.y * source.height),
    width: Math.round(rect.width * source.width),
    height: Math.round(rect.height * source.height),
  };
}

function assertPositiveSize(size: Size): void {
  if (
    !Number.isFinite(size.width) ||
    !Number.isFinite(size.height) ||
    size.width <= 0 ||
    size.height <= 0
  ) {
    throw new Error("Sizes must contain positive finite dimensions");
  }
}

function clamp(value: number, minimum: number, maximum: number): number {
  return Math.min(maximum, Math.max(minimum, value));
}

function roundSix(value: number): number {
  return Math.round(value * 1_000_000) / 1_000_000;
}
