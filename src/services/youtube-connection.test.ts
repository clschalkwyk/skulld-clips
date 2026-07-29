import { describe, expect, it, vi } from "vitest";

import type { YouTubeConnectionStatus } from "../../contracts/types";
import {
  isYouTubeConnectionPending,
  pollYouTubeConnectionStatus,
  youtubeConnectionPhaseLabel,
} from "./youtube-connection";

function status(
  connectionPhase: YouTubeConnectionStatus["connectionPhase"],
  authenticated = false,
): YouTubeConnectionStatus {
  return {
    configured: true,
    authenticated,
    connectionPhase,
    channel: authenticated ? { channelId: "channel-id", title: "Channel" } : null,
    lastSyncedAt: null,
  };
}

describe("YouTube connection polling", () => {
  it("reports the native connection phases with actionable labels", () => {
    expect(isYouTubeConnectionPending("awaitingBrowser")).toBe(true);
    expect(isYouTubeConnectionPending("exchangingToken")).toBe(true);
    expect(isYouTubeConnectionPending("loadingChannel")).toBe(true);
    expect(isYouTubeConnectionPending("failed")).toBe(false);
    expect(youtubeConnectionPhaseLabel("loadingChannel")).toContain(
      "authorized channel",
    );
  });

  it("polls through browser and token phases until the channel is connected", async () => {
    const statuses = [
      status("disconnected"),
      status("awaitingBrowser"),
      status("exchangingToken"),
      status("loadingChannel"),
      status("connected", true),
    ];
    const readStatus = vi.fn(async () => statuses.shift() ?? status("failed"));
    const onStatus = vi.fn();

    const result = await pollYouTubeConnectionStatus({
      readStatus,
      onStatus,
      signal: new AbortController().signal,
      intervalMs: 0,
      timeoutMs: 1_000,
    });

    expect(result?.authenticated).toBe(true);
    expect(readStatus).toHaveBeenCalledTimes(5);
    expect(onStatus).toHaveBeenLastCalledWith(status("connected", true));
  });

  it("stops cleanly when the panel closes", async () => {
    const controller = new AbortController();
    controller.abort();
    const readStatus = vi.fn(async () => status("connected", true));

    const result = await pollYouTubeConnectionStatus({
      readStatus,
      onStatus: vi.fn(),
      signal: controller.signal,
      intervalMs: 0,
      timeoutMs: 1_000,
    });

    expect(result).toBeNull();
    expect(readStatus).not.toHaveBeenCalled();
  });
});
