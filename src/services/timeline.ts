import type { MediaProbe } from "../../contracts/types";

export const MIN_TRIM_DURATION_MS = 250;

export function setTrimIn(
  requestedMs: number,
  outMs: number,
  durationMs: number,
): number {
  return integerMs(
    clamp(requestedMs, 0, Math.min(durationMs, outMs) - MIN_TRIM_DURATION_MS),
  );
}

export function setTrimOut(
  requestedMs: number,
  inMs: number,
  durationMs: number,
): number {
  return integerMs(
    clamp(requestedMs, inMs + MIN_TRIM_DURATION_MS, durationMs),
  );
}

export function clampPlayhead(
  requestedMs: number,
  inMs: number,
  outMs: number,
): number {
  return integerMs(clamp(requestedMs, inMs, outMs));
}

export function approximateFrameStepMs(probe: MediaProbe): number {
  const rate =
    probe.video.avgFrameRate ?? probe.video.realFrameRate ?? 30;
  return Math.max(1, Math.round(1_000 / Math.min(240, Math.max(1, rate))));
}

export function formatTimelineTime(milliseconds: number): string {
  const safe = Math.max(0, integerMs(milliseconds));
  const minutes = Math.floor(safe / 60_000);
  const seconds = Math.floor((safe % 60_000) / 1_000);
  const millis = safe % 1_000;
  return `${minutes.toString().padStart(2, "0")}:${seconds
    .toString()
    .padStart(2, "0")}.${millis.toString().padStart(3, "0")}`;
}

export function isTypingTarget(target: EventTarget | null): boolean {
  return (
    target instanceof HTMLInputElement ||
    target instanceof HTMLTextAreaElement ||
    target instanceof HTMLSelectElement ||
    (target instanceof HTMLElement && target.isContentEditable)
  );
}

function integerMs(value: number): number {
  return Math.round(Number.isFinite(value) ? value : 0);
}

function clamp(value: number, minimum: number, maximum: number): number {
  return Math.min(maximum, Math.max(minimum, value));
}
