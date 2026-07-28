export interface RelativeOverlayTiming {
  startOffsetMs: number;
  durationMs: number;
  leftPercent: number;
  widthPercent: number;
}

export interface OverlayTimingRange {
  startMs: number;
  endMs: number;
}

export function relativeOverlayTiming(
  startMs: number,
  endMs: number,
  clipInMs: number,
  clipOutMs: number,
): RelativeOverlayTiming {
  const rangeMs = Math.max(1, clipOutMs - clipInMs);
  const visibleStartMs = clamp(startMs, clipInMs, clipOutMs);
  const visibleEndMs = clamp(endMs, visibleStartMs, clipOutMs);

  return {
    startOffsetMs: Math.round(startMs - clipInMs),
    durationMs: Math.max(0, Math.round(endMs - startMs)),
    leftPercent: ((visibleStartMs - clipInMs) / rangeMs) * 100,
    widthPercent: ((visibleEndMs - visibleStartMs) / rangeMs) * 100,
  };
}

export function formatRelativeSeconds(milliseconds: number): string {
  const sign = milliseconds < 0 ? "−" : "";
  const fixed = (Math.abs(milliseconds) / 1_000).toFixed(3);
  const [whole, fraction = "000"] = fixed.split(".");
  const conciseFraction = fraction.replace(/0+$/, "").padEnd(2, "0");
  return `${sign}${whole}.${conciseFraction}s`;
}

export function placeOverlayStartAtPlayhead(
  playheadMs: number,
  currentStartMs: number,
  currentEndMs: number,
  clipInMs: number,
  clipOutMs: number,
  minimumDurationMs: number,
  maximumDurationMs = Number.POSITIVE_INFINITY,
): OverlayTimingRange {
  const durationMs = boundedDuration(
    currentEndMs - currentStartMs,
    clipOutMs - clipInMs,
    minimumDurationMs,
    maximumDurationMs,
  );
  const startMs = Math.round(
    clamp(playheadMs, clipInMs, clipOutMs - minimumDurationMs),
  );
  return {
    startMs,
    endMs: Math.round(
      clamp(startMs + durationMs, startMs + minimumDurationMs, clipOutMs),
    ),
  };
}

export function placeOverlayEndAtPlayhead(
  playheadMs: number,
  currentStartMs: number,
  currentEndMs: number,
  clipInMs: number,
  clipOutMs: number,
  minimumDurationMs: number,
  maximumDurationMs = Number.POSITIVE_INFINITY,
): OverlayTimingRange {
  const durationMs = boundedDuration(
    currentEndMs - currentStartMs,
    clipOutMs - clipInMs,
    minimumDurationMs,
    maximumDurationMs,
  );
  const endMs = Math.round(
    clamp(playheadMs, clipInMs + minimumDurationMs, clipOutMs),
  );
  return {
    startMs: Math.round(
      clamp(endMs - durationMs, clipInMs, endMs - minimumDurationMs),
    ),
    endMs,
  };
}

export function placeOverlayAtStartOffset(
  requestedOffsetMs: number,
  currentStartMs: number,
  currentEndMs: number,
  clipInMs: number,
  clipOutMs: number,
): OverlayTimingRange {
  const durationMs = boundedDuration(
    currentEndMs - currentStartMs,
    clipOutMs - clipInMs,
    1,
    Number.POSITIVE_INFINITY,
  );
  const startMs = Math.round(
    clamp(clipInMs + requestedOffsetMs, clipInMs, clipOutMs - durationMs),
  );
  return {
    startMs,
    endMs: startMs + durationMs,
  };
}

function boundedDuration(
  requestedDurationMs: number,
  clipDurationMs: number,
  minimumDurationMs: number,
  maximumDurationMs: number,
): number {
  const maximum = Math.max(
    minimumDurationMs,
    Math.min(clipDurationMs, maximumDurationMs),
  );
  return Math.round(clamp(requestedDurationMs, minimumDurationMs, maximum));
}

function clamp(value: number, minimum: number, maximum: number): number {
  const finite = Number.isFinite(value) ? value : minimum;
  return Math.min(maximum, Math.max(minimum, finite));
}
