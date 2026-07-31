<script lang="ts">
  import { onMount } from "svelte";

  import type {
    AppError,
    YouTubeConnectionStatus,
    YouTubePerformanceMetrics,
    YouTubeProjectPerformance,
    YouTubeVideoCandidate,
  } from "../../../contracts/types";
  import {
    connectYouTubeChannel,
    disconnectYouTubeChannel,
    getYouTubeConnectionStatus,
    linkProjectToYouTubeVideo,
    listRecentYouTubeUploads,
    listYouTubePerformance,
    normalizeAppError,
    syncYouTubePerformance,
  } from "../../services/tauri";
  import {
    isYouTubeConnectionPending,
    pollYouTubeConnectionStatus,
    youtubeConnectionPhaseLabel,
  } from "../../services/youtube-connection";

  interface Props {
    project: { id: string; name: string } | null;
    onClose: () => void;
  }

  let { project, onClose }: Props = $props();

  let status = $state<YouTubeConnectionStatus | null>(null);
  let links = $state<YouTubeProjectPerformance[]>([]);
  let uploads = $state<YouTubeVideoCandidate[]>([]);
  let loading = $state(true);
  let busyAction = $state<"connect" | "disconnect" | "load" | "link" | "sync" | null>(null);
  let error = $state<AppError | null>(null);
  let videoUrl = $state("");
  let disconnectArmed = $state(false);
  let closeButton: HTMLButtonElement;
  let selectedProjectId = $state<string | null>(null);
  let relinking = $state(false);
  let connectionPoll: AbortController | null = null;
  let connectRequestActive = false;
  let disposed = false;

  const activeLink = $derived(
    project
      ? links.find((link) => link.projectId === project.id) ?? null
      : links.find((link) => link.projectId === selectedProjectId) ??
        links[0] ??
        null,
  );
  const performance = $derived(activeLink?.performance ?? null);
  const integrationUnavailable = $derived(
    !loading && status?.configured === false,
  );

  onMount(() => {
    const previouslyFocusedElement =
      document.activeElement instanceof HTMLElement
        ? document.activeElement
        : null;
    closeButton.focus();
    void loadWorkspace();
    return () => {
      disposed = true;
      stopConnectionPolling();
      previouslyFocusedElement?.focus();
    };
  });

  async function loadWorkspace(): Promise<void> {
    loading = true;
    error = null;
    try {
      const connection = await getYouTubeConnectionStatus();
      status = connection;
      if (!connection.configured) {
        links = [];
        return;
      }
      links = await listYouTubePerformance();
      if (connection.authenticated) {
        await loadUploads();
      } else if (isYouTubeConnectionPending(connection.connectionPhase)) {
        busyAction = "connect";
        startConnectionPolling();
      }
    } catch (caught) {
      error = normalizeAppError(caught);
    } finally {
      loading = false;
    }
  }

  async function connect(): Promise<void> {
    busyAction = "connect";
    error = null;
    connectRequestActive = true;
    startConnectionPolling();
    try {
      const connection = await connectYouTubeChannel();
      if (disposed) {
        return;
      }
      status = connection;
      links = [];
      await loadUploads();
    } catch (caught) {
      if (!disposed) {
        error = normalizeAppError(caught);
        try {
          status = await getYouTubeConnectionStatus();
        } catch {
          // Preserve the actionable error returned by the connect command.
        }
      }
    } finally {
      connectRequestActive = false;
      stopConnectionPolling();
      if (!disposed) {
        busyAction = null;
      }
    }
  }

  function startConnectionPolling(): void {
    stopConnectionPolling();
    const controller = new AbortController();
    connectionPoll = controller;
    void pollYouTubeConnectionStatus({
      readStatus: getYouTubeConnectionStatus,
      onStatus: (connection) => {
        if (!disposed && connectionPoll === controller) {
          status = connection;
        }
      },
      signal: controller.signal,
    }).then((connection) => {
      if (
        disposed ||
        connectionPoll !== controller ||
        connectRequestActive
      ) {
        return;
      }
      connectionPoll = null;
      busyAction = null;
      if (connection?.authenticated) {
        void loadUploads();
      }
    });
  }

  function stopConnectionPolling(): void {
    connectionPoll?.abort();
    connectionPoll = null;
  }

  async function disconnect(): Promise<void> {
    if (!disconnectArmed) {
      disconnectArmed = true;
      return;
    }
    busyAction = "disconnect";
    error = null;
    try {
      status = await disconnectYouTubeChannel();
      links = [];
      uploads = [];
      disconnectArmed = false;
    } catch (caught) {
      error = normalizeAppError(caught);
    } finally {
      busyAction = null;
    }
  }

  async function loadUploads(): Promise<void> {
    busyAction = "load";
    try {
      uploads = await listRecentYouTubeUploads();
    } catch (caught) {
      error = normalizeAppError(caught);
    } finally {
      busyAction = null;
    }
  }

  async function linkVideo(value: string): Promise<void> {
    if (!project) {
      return;
    }
    busyAction = "link";
    error = null;
    try {
      await linkProjectToYouTubeVideo(project.id, project.name, value);
      links = await syncYouTubePerformance(project.id);
      videoUrl = "";
      relinking = false;
    } catch (caught) {
      error = normalizeAppError(caught);
    } finally {
      busyAction = null;
    }
  }

  async function sync(projectId?: string): Promise<void> {
    busyAction = "sync";
    error = null;
    try {
      links = await syncYouTubePerformance(projectId);
      status = await getYouTubeConnectionStatus();
    } catch (caught) {
      error = normalizeAppError(caught);
    } finally {
      busyAction = null;
    }
  }

  function submitUrl(event: SubmitEvent): void {
    event.preventDefault();
    if (videoUrl.trim()) {
      void linkVideo(videoUrl);
    }
  }

  function formatCount(value: number): string {
    return new Intl.NumberFormat(undefined, {
      notation: value >= 10_000 ? "compact" : "standard",
      maximumFractionDigits: value >= 10_000 ? 1 : 0,
    }).format(value);
  }

  function formatDuration(seconds: number): string {
    const rounded = Math.max(0, Math.round(seconds));
    const minutes = Math.floor(rounded / 60);
    return `${minutes}:${String(rounded % 60).padStart(2, "0")}`;
  }

  function formatWatchTime(minutes: number): string {
    const hours = minutes / 60;
    return hours >= 10
      ? `${Math.round(hours).toLocaleString()} h`
      : `${hours.toFixed(1)} h`;
  }

  function netSubscribers(metrics: YouTubePerformanceMetrics): number {
    return metrics.subscribersGained - metrics.subscribersLost;
  }

  function formatDate(value: string): string {
    const date = new Date(value);
    return Number.isNaN(date.valueOf())
      ? "Date unavailable"
      : date.toLocaleDateString(undefined, {
          day: "numeric",
          month: "short",
          year: "numeric",
        });
  }
</script>

<div class="performance-backdrop" role="presentation">
  <div
    class="performance-dialog"
    class:unavailable={integrationUnavailable}
    role="dialog"
    aria-modal="true"
    aria-labelledby="performance-heading"
  >
    <header class="performance-header">
      <div>
        <p class="section-label">YouTube owner analytics</p>
        <h2 id="performance-heading">Channel performance</h2>
      </div>
      <button
        class="text-button performance-close"
        type="button"
        aria-label="Close channel performance"
        onclick={onClose}
        bind:this={closeButton}
      >
        Close
      </button>
    </header>

    {#if loading}
      <div class="performance-loading" aria-live="polite">
        <span class="spinner" aria-hidden="true"></span>
        Loading local performance data…
      </div>
    {:else if !status}
      <div class="performance-empty" role="status">
        <span class="performance-kicker">Workspace unavailable</span>
        <h3>Channel performance couldn’t open.</h3>
        <p>
          Close this panel and try again. Your editor and local projects are
          unaffected.
        </p>
      </div>
    {:else if !status.configured}
      <section
        class="performance-unavailable"
        aria-labelledby="performance-unavailable-heading"
      >
        <div class="performance-unavailable-mark" aria-hidden="true">
          <svg viewBox="0 0 24 24" focusable="false">
            <path d="M10 8.75 15 12l-5 3.25z"></path>
          </svg>
        </div>
        <div class="performance-unavailable-message" role="status" aria-live="polite">
          <span class="performance-kicker">Optional integration</span>
          <h3 id="performance-unavailable-heading">
            Channel performance isn’t available in this build.
          </h3>
          <p class="performance-unavailable-copy">
            You can keep editing and exporting normally. YouTube analytics is a
            separate read-only feature and does not affect your local projects.
          </p>
        </div>
        <details class="performance-setup">
          <summary>Developer setup</summary>
          <div>
            <p>
              Configure a Google OAuth desktop client before starting Clip
              Forge, enable the YouTube Data and YouTube Analytics APIs, then
              restart the app.
            </p>
            <code>SKCF_YOUTUBE_CLIENT_ID</code>
          </div>
        </details>
        <button class="primary-button" type="button" onclick={onClose}>
          Back to Clip Forge
        </button>
      </section>
    {:else if !status.authenticated}
      <div class="performance-empty">
        <span class="performance-kicker">No channel connected</span>
        <h3>Measure what happens after the export.</h3>
        <p>
          Connect with read-only access. Clip Forge stores the refresh token in
          the operating-system credential store and never uploads media.
        </p>
        {#if busyAction === "connect"}
          <div class="performance-connection-status" role="status" aria-live="polite">
            <span class="spinner" aria-hidden="true"></span>
            <span>
              {youtubeConnectionPhaseLabel(status.connectionPhase)}
            </span>
          </div>
        {/if}
        <button
          class="primary-button"
          type="button"
          onclick={connect}
          disabled={busyAction !== null}
        >
          {busyAction === "connect" ? "Connection in progress…" : "Connect YouTube channel"}
        </button>
      </div>
    {:else}
      <div class="performance-channel-bar">
        <div>
          <span>Connected channel</span>
          <strong>{status.channel?.title ?? "YouTube channel"}</strong>
        </div>
        <div class="performance-channel-actions">
          <button
            class="secondary-button"
            type="button"
            onclick={() => sync()}
            disabled={busyAction !== null || links.length === 0}
          >
            {busyAction === "sync" ? "Refreshing…" : "Refresh performance"}
          </button>
          <button
            class:danger={disconnectArmed}
            class="text-button"
            type="button"
            onclick={disconnect}
            disabled={busyAction !== null}
          >
            {busyAction === "disconnect"
              ? "Clearing…"
              : disconnectArmed
                ? "Confirm disconnect + clear"
                : "Disconnect"}
          </button>
        </div>
      </div>

      {#if project && (!activeLink || relinking)}
        <section class="performance-linker" aria-labelledby="link-heading">
          <div>
            <p class="section-label">{activeLink ? "Change linked video" : "Link this project"}</p>
            <h3 id="link-heading">{project.name}</h3>
            <p>
              Select the upload created from this project. Clip Forge will not
              guess from filenames or titles.
            </p>
            {#if activeLink}
              <button class="text-button" type="button" onclick={() => (relinking = false)}>
                Keep current link
              </button>
            {/if}
          </div>
          <form class="performance-url-form" onsubmit={submitUrl}>
            <label for="youtube-video-url">YouTube URL or video ID</label>
            <div>
              <input
                id="youtube-video-url"
                type="text"
                placeholder="https://youtube.com/shorts/…"
                bind:value={videoUrl}
                disabled={busyAction !== null}
              />
              <button
                class="secondary-button"
                type="submit"
                disabled={busyAction !== null || !videoUrl.trim()}
              >
                {busyAction === "link" ? "Linking…" : "Link video"}
              </button>
            </div>
          </form>
          <div class="performance-upload-list" aria-label="Recent channel uploads">
            {#if busyAction === "load"}
              <p>Loading recent uploads…</p>
            {:else if uploads.length === 0}
              <p>No recent uploads are available. Paste the video URL above.</p>
            {:else}
              {#each uploads.slice(0, 8) as upload (upload.videoId)}
                <article>
                  <div>
                    <strong>{upload.title}</strong>
                    <span>{formatDate(upload.publishedAt)}</span>
                  </div>
                  <button
                    class="text-button"
                    type="button"
                    onclick={() => linkVideo(upload.videoId)}
                    disabled={busyAction !== null}
                  >
                    Link
                  </button>
                </article>
              {/each}
            {/if}
          </div>
        </section>
      {:else if activeLink}
        <section class="performance-scorecard" aria-labelledby="scorecard-heading">
          <div class="performance-scorecard-heading">
            <div>
              <p class="section-label">{activeLink.projectName}</p>
              <h3 id="scorecard-heading">{activeLink.videoTitle}</h3>
              <p>
                Published {formatDate(activeLink.publishedAt)}
                {#if performance}
                  · data through {formatDate(performance.endDate)}
                {/if}
              </p>
            </div>
            <div class="performance-scorecard-actions">
              <button
                class="secondary-button"
                type="button"
                onclick={() => sync(activeLink.projectId)}
                disabled={busyAction !== null}
              >
                {busyAction === "sync" ? "Refreshing…" : "Refresh video"}
              </button>
              {#if project}
                <button
                  class="text-button"
                  type="button"
                  onclick={() => (relinking = true)}
                  disabled={busyAction !== null}
                >
                  Change link
                </button>
              {/if}
            </div>
          </div>

          {#if performance}
            <dl class="metric-grid">
              <div>
                <dt>Engaged views</dt>
                <dd>{formatCount(performance.metrics.engagedViews)}</dd>
                <span>{formatCount(performance.metrics.views)} total views</span>
              </div>
              <div>
                <dt>Watch time</dt>
                <dd>{formatWatchTime(performance.metrics.estimatedMinutesWatched)}</dd>
                <span>{formatDuration(performance.metrics.averageViewDurationSeconds)} average</span>
              </div>
              <div>
                <dt>Average viewed</dt>
                <dd>{performance.metrics.averageViewPercentage.toFixed(1)}%</dd>
                <span>owner analytics</span>
              </div>
              <div>
                <dt>Interactions</dt>
                <dd>
                  {formatCount(
                    performance.metrics.likes +
                      performance.metrics.comments +
                      performance.metrics.shares,
                  )}
                </dd>
                <span>
                  {formatCount(performance.metrics.likes)} likes ·
                  {formatCount(performance.metrics.shares)} shares
                </span>
              </div>
              <div>
                <dt>Net subscribers</dt>
                <dd>
                  {netSubscribers(performance.metrics) > 0 ? "+" : ""}{formatCount(
                    netSubscribers(performance.metrics),
                  )}
                </dd>
                <span>
                  +{formatCount(performance.metrics.subscribersGained)} ·
                  −{formatCount(performance.metrics.subscribersLost)}
                </span>
              </div>
            </dl>

            {#if performance.daily.length > 0}
              <div class="performance-table-wrap">
                <table>
                  <caption>Latest daily performance</caption>
                  <thead>
                    <tr>
                      <th scope="col">Date</th>
                      <th scope="col">Engaged views</th>
                      <th scope="col">Watch time</th>
                      <th scope="col">Average viewed</th>
                      <th scope="col">Net subs</th>
                    </tr>
                  </thead>
                  <tbody>
                    {#each performance.daily.slice(-14).reverse() as day (day.date)}
                      <tr>
                        <th scope="row">{formatDate(day.date)}</th>
                        <td>{formatCount(day.metrics.engagedViews)}</td>
                        <td>{formatWatchTime(day.metrics.estimatedMinutesWatched)}</td>
                        <td>{day.metrics.averageViewPercentage.toFixed(1)}%</td>
                        <td>
                          {netSubscribers(day.metrics) > 0 ? "+" : ""}{formatCount(
                            netSubscribers(day.metrics),
                          )}
                        </td>
                      </tr>
                    {/each}
                  </tbody>
                </table>
              </div>
            {:else}
              <div class="performance-pending">
                YouTube has not published daily analytics for this video yet.
              </div>
            {/if}
          {:else}
            <div class="performance-pending">
              This video is linked. Refresh it to pull the first owner analytics
              snapshot.
            </div>
          {/if}
        </section>
      {:else}
        <div class="performance-empty">
          <span class="performance-kicker">No linked Clip Forge videos</span>
          <h3>Open a project to link its YouTube upload.</h3>
          <p>
            A project must be linked explicitly because the upload happens
            outside Clip Forge.
          </p>
        </div>
      {/if}

      {#if !project && links.length > 1}
        <section class="performance-library" aria-labelledby="library-heading">
          <div class="section-heading">
            <div>
              <p class="section-label">Tracked exports</p>
              <h3 id="library-heading">Linked videos</h3>
            </div>
          </div>
          {#each links as link (link.projectId)}
            <article class:active={activeLink?.projectId === link.projectId}>
              <div>
                <strong>{link.videoTitle}</strong>
                <span>{link.projectName} · {formatDate(link.publishedAt)}</span>
              </div>
              <div class="performance-library-action">
                <span>
                  {link.performance
                    ? `${formatCount(link.performance.metrics.engagedViews)} engaged views`
                    : "Awaiting first refresh"}
                </span>
                <button
                  class="text-button"
                  type="button"
                  onclick={() => (selectedProjectId = link.projectId)}
                >
                  View
                </button>
              </div>
            </article>
          {/each}
        </section>
      {/if}
    {/if}

    {#if error && !(integrationUnavailable && error.code === "E_INTEGRATION_UNAVAILABLE")}
      <div class="performance-error" role="alert" aria-live="assertive">
        <div>
          <strong>{error.message}</strong>
          {#if error.safeDetail}<p>{error.safeDetail}</p>{/if}
        </div>
        <code>{error.code}</code>
      </div>
    {/if}

    {#if !integrationUnavailable}
      <footer class="performance-footer">
        <span>Read-only · no media upload</span>
        <span>YouTube data may be delayed</span>
      </footer>
    {/if}
  </div>
</div>
