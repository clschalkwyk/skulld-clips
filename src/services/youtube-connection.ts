import type {
  YouTubeConnectionPhase,
  YouTubeConnectionStatus,
} from "../../contracts/types";

const DEFAULT_POLL_INTERVAL_MS = 1_000;
const DEFAULT_POLL_TIMEOUT_MS = 185_000;

export function isYouTubeConnectionPending(
  phase: YouTubeConnectionPhase,
): boolean {
  return (
    phase === "awaitingBrowser" ||
    phase === "exchangingToken" ||
    phase === "loadingChannel"
  );
}

export function youtubeConnectionPhaseLabel(
  phase: YouTubeConnectionPhase,
): string {
  switch (phase) {
    case "awaitingBrowser":
      return "Waiting for approval in your browser…";
    case "exchangingToken":
      return "Securing YouTube access…";
    case "loadingChannel":
      return "Loading the authorized channel…";
    case "connected":
      return "YouTube channel connected.";
    case "failed":
      return "Connection did not complete. Review the error and retry.";
    default:
      return "Starting YouTube connection…";
  }
}

interface PollYouTubeConnectionOptions {
  readStatus: () => Promise<YouTubeConnectionStatus>;
  onStatus: (status: YouTubeConnectionStatus) => void;
  signal: AbortSignal;
  intervalMs?: number;
  timeoutMs?: number;
}

export async function pollYouTubeConnectionStatus({
  readStatus,
  onStatus,
  signal,
  intervalMs = DEFAULT_POLL_INTERVAL_MS,
  timeoutMs = DEFAULT_POLL_TIMEOUT_MS,
}: PollYouTubeConnectionOptions): Promise<YouTubeConnectionStatus | null> {
  const deadline = Date.now() + timeoutMs;
  while (!signal.aborted && Date.now() < deadline) {
    try {
      const status = await readStatus();
      if (signal.aborted) {
        return null;
      }
      onStatus(status);
      if (
        status.authenticated ||
        status.connectionPhase === "connected" ||
        status.connectionPhase === "failed"
      ) {
        return status;
      }
    } catch {
      // The native connect command remains authoritative; retry status reads
      // until it completes, the panel closes, or the bounded timeout expires.
    }
    await waitForNextPoll(intervalMs, signal);
  }
  return null;
}

function waitForNextPoll(milliseconds: number, signal: AbortSignal): Promise<void> {
  return new Promise((resolve) => {
    if (signal.aborted) {
      resolve();
      return;
    }
    const finish = (): void => {
      clearTimeout(timer);
      signal.removeEventListener("abort", finish);
      resolve();
    };
    const timer = setTimeout(finish, Math.max(0, milliseconds));
    signal.addEventListener("abort", finish, { once: true });
  });
}
