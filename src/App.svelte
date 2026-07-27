<script lang="ts">
  import { onMount } from "svelte";

  import type {
    AppError,
    LoadProjectResult,
    NormalizedRect,
    RecentProject,
  } from "../contracts/types";
  import CropInspector from "./components/editor/CropInspector.svelte";
  import PreviewStage from "./components/editor/PreviewStage.svelte";
  import Timeline from "./components/editor/Timeline.svelte";
  import type { RuntimeInfo } from "./contracts/runtime";
  import { createAutosaveScheduler, type AutosaveScheduler } from "./services/autosave";
  import {
    createProject,
    getRuntimeInfo,
    listRecentProjects,
    listenForFileDrops,
    loadProject,
    mediaPreviewUrl,
    normalizeAppError,
    relinkSource,
    removeRecentProject,
    saveProject,
    selectMediaFile,
    selectProjectFile,
  } from "./services/tauri";
  import {
    approximateFrameStepMs,
    clampPlayhead,
    isTypingTarget,
    setTrimIn,
    setTrimOut,
  } from "./services/timeline";

  type BusyAction = "importing" | "opening" | "relinking" | null;
  type SaveState = "saved" | "unsaved" | "saving" | "error";

  let runtime = $state<RuntimeInfo | null>(null);
  let runtimeError = $state<AppError | null>(null);
  let recents = $state<RecentProject[]>([]);
  let recentsLoading = $state(true);
  let session = $state<LoadProjectResult | null>(null);
  let actionError = $state<AppError | null>(null);
  let busyAction = $state<BusyAction>(null);
  let dropActive = $state(false);
  let saveState = $state<SaveState>("saved");
  let pendingReplacementPath = $state<string | null>(null);
  let playheadMs = $state(0);
  let playing = $state(false);
  let previewError = $state<string | null>(null);

  let autosave: AutosaveScheduler | null = null;
  let unlistenDrop: (() => void) | null = null;

  const sourceStatusLabel = $derived(
    session?.sourceStatus === "missing"
      ? "Source missing"
      : session?.sourceStatus === "changed"
        ? "Source changed"
        : "Source verified",
  );
  const sourceSize = $derived({
    width: session?.project.source.probe.video.displayWidth ?? 1,
    height: session?.project.source.probe.video.displayHeight ?? 1,
  });
  const previewUrl = $derived(
    session?.sourceStatus === "ok"
      ? mediaPreviewUrl(session.project.source.path)
      : null,
  );

  onMount(() => {
    void bootstrap();
    return () => {
      unlistenDrop?.();
      autosave?.dispose();
    };
  });

  async function bootstrap(): Promise<void> {
    const [runtimeResult] = await Promise.allSettled([
      loadRuntime(),
      refreshRecents(),
    ]);
    if (runtimeResult.status === "rejected") {
      runtimeError = normalizeAppError(runtimeResult.reason);
    }
    try {
      unlistenDrop = await listenForFileDrops(
        (paths) => {
          if (!session && paths[0]) {
            void importClip(paths[0]);
          }
        },
        (active) => {
          dropActive = active;
        },
      );
    } catch (error) {
      runtimeError = normalizeAppError(error);
    }
  }

  async function loadRuntime(): Promise<void> {
    runtimeError = null;
    try {
      runtime = await getRuntimeInfo();
    } catch (error) {
      runtime = null;
      runtimeError = normalizeAppError(error);
    }
  }

  async function refreshRecents(): Promise<void> {
    recentsLoading = true;
    try {
      recents = await listRecentProjects();
    } catch (error) {
      actionError = normalizeAppError(error);
    } finally {
      recentsLoading = false;
    }
  }

  async function chooseClip(): Promise<void> {
    actionError = null;
    try {
      const path = await selectMediaFile();
      if (path) {
        await importClip(path);
      }
    } catch (error) {
      actionError = normalizeAppError(error);
    }
  }

  async function importClip(path: string): Promise<void> {
    busyAction = "importing";
    actionError = null;
    try {
      const created = await createProject(path);
      openSession({
        ...created,
        sourceStatus: "ok",
        migrationApplied: false,
      });
    } catch (error) {
      actionError = normalizeAppError(error);
    } finally {
      busyAction = null;
    }
  }

  async function chooseProject(): Promise<void> {
    actionError = null;
    try {
      const path = await selectProjectFile();
      if (path) {
        await openProject(path);
      }
    } catch (error) {
      actionError = normalizeAppError(error);
    }
  }

  async function openProject(projectPath: string): Promise<void> {
    busyAction = "opening";
    actionError = null;
    try {
      openSession(await loadProject(projectPath));
    } catch (error) {
      actionError = normalizeAppError(error);
    } finally {
      busyAction = null;
    }
  }

  function openSession(next: LoadProjectResult): void {
    autosave?.dispose();
    session = next;
    pendingReplacementPath = null;
    playheadMs = next.project.timeline.inMs;
    playing = false;
    previewError = null;
    saveState = "saved";
    autosave = createAutosaveScheduler(persistProject);
  }

  function updateProjectName(event: Event): void {
    if (!session) {
      return;
    }
    session.project.name = (event.currentTarget as HTMLInputElement).value;
    markProjectDirty();
  }

  function markProjectDirty(): void {
    saveState = "unsaved";
    autosave?.markDirty();
  }

  function updateCrop(crop: NormalizedRect): void {
    if (!session || session.sourceStatus !== "ok") {
      return;
    }
    session.project.crop = crop;
    markProjectDirty();
  }

  function updateTrimIn(requestedMs: number): void {
    if (!session) {
      return;
    }
    const timeline = session.project.timeline;
    timeline.inMs = setTrimIn(
      requestedMs,
      timeline.outMs,
      session.project.source.probe.durationMs,
    );
    playheadMs = clampPlayhead(playheadMs, timeline.inMs, timeline.outMs);
    markProjectDirty();
  }

  function updateTrimOut(requestedMs: number): void {
    if (!session) {
      return;
    }
    const timeline = session.project.timeline;
    timeline.outMs = setTrimOut(
      requestedMs,
      timeline.inMs,
      session.project.source.probe.durationMs,
    );
    playheadMs = clampPlayhead(playheadMs, timeline.inMs, timeline.outMs);
    markProjectDirty();
  }

  function updatePlayhead(requestedMs: number): void {
    if (!session) {
      return;
    }
    playheadMs = clampPlayhead(
      requestedMs,
      session.project.timeline.inMs,
      session.project.timeline.outMs,
    );
  }

  function updatePlaying(next: boolean): void {
    if (!session || session.sourceStatus !== "ok") {
      playing = false;
      return;
    }
    if (next && playheadMs >= session.project.timeline.outMs) {
      playheadMs = session.project.timeline.inMs;
    }
    playing = next;
  }

  async function persistProject(): Promise<void> {
    const active = session;
    if (!active) {
      return;
    }
    saveState = "saving";
    actionError = null;
    try {
      const saved = await saveProject(active.projectPath, active.project);
      if (session?.projectPath === active.projectPath) {
        session.project.updatedAt = saved.savedAt;
        saveState = "saved";
      }
    } catch (error) {
      if (session?.projectPath === active.projectPath) {
        saveState = "error";
        actionError = normalizeAppError(error);
      }
    }
  }

  async function backToHome(): Promise<void> {
    playing = false;
    await autosave?.flush();
    autosave?.dispose();
    autosave = null;
    session = null;
    pendingReplacementPath = null;
    await refreshRecents();
  }

  async function chooseReplacement(): Promise<void> {
    if (!session) {
      return;
    }
    actionError = null;
    try {
      const path = await selectMediaFile();
      if (path) {
        await applyReplacement(path, false);
      }
    } catch (error) {
      actionError = normalizeAppError(error);
    }
  }

  async function applyReplacement(
    replacementPath: string,
    acceptMismatch: boolean,
  ): Promise<void> {
    if (!session) {
      return;
    }
    busyAction = "relinking";
    actionError = null;
    try {
      const result = await relinkSource(
        session.projectPath,
        replacementPath,
        acceptMismatch,
      );
      session.project = result.project;
      session.sourceStatus = "ok";
      playheadMs = result.project.timeline.inMs;
      playing = false;
      previewError = null;
      pendingReplacementPath = null;
      saveState = "saved";
      await refreshRecents();
    } catch (error) {
      const normalized = normalizeAppError(error);
      if (normalized.code === "E_SOURCE_CHANGED" && !acceptMismatch) {
        pendingReplacementPath = replacementPath;
      }
      actionError = normalized;
    } finally {
      busyAction = null;
    }
  }

  async function removeRecent(path: string): Promise<void> {
    actionError = null;
    try {
      await removeRecentProject(path);
      recents = recents.filter((recent) => recent.projectPath !== path);
    } catch (error) {
      actionError = normalizeAppError(error);
    }
  }

  function formatDuration(durationMs: number): string {
    const totalSeconds = Math.round(durationMs / 1_000);
    const minutes = Math.floor(totalSeconds / 60);
    const seconds = totalSeconds % 60;
    return `${minutes}:${seconds.toString().padStart(2, "0")}`;
  }

  function formatDate(value: string): string {
    const date = new Date(value);
    return Number.isNaN(date.valueOf())
      ? "Last opened time unavailable"
      : date.toLocaleString(undefined, {
          dateStyle: "medium",
          timeStyle: "short",
        });
  }

  function handleShortcut(event: KeyboardEvent): void {
    if (!session || (isTypingTarget(event.target) && event.key !== "Escape")) {
      return;
    }
    const timeline = session.project.timeline;
    const modifier = event.metaKey || event.ctrlKey;
    if (modifier && event.key.toLowerCase() === "s") {
      event.preventDefault();
      void autosave?.flush();
      return;
    }
    switch (event.key) {
      case " ":
        event.preventDefault();
        updatePlaying(!playing);
        break;
      case "i":
      case "I":
        event.preventDefault();
        updateTrimIn(playheadMs);
        break;
      case "o":
      case "O":
        event.preventDefault();
        updateTrimOut(playheadMs);
        break;
      case "ArrowLeft":
      case "ArrowRight": {
        event.preventDefault();
        const direction = event.key === "ArrowLeft" ? -1 : 1;
        const delta = event.shiftKey
          ? 1_000
          : approximateFrameStepMs(session.project.source.probe);
        updatePlayhead(playheadMs + direction * delta);
        break;
      }
      case "Home":
        event.preventDefault();
        updatePlayhead(timeline.inMs);
        break;
      case "End":
        event.preventDefault();
        updatePlayhead(timeline.outMs);
        break;
    }
  }
</script>

<svelte:head>
  <title>Skull’d Clip Forge</title>
</svelte:head>
<svelte:window onkeydown={handleShortcut} />

{#if session}
  <main class="editor-shell">
    <header class="editor-bar">
      <button class="text-button" type="button" onclick={backToHome}>← Projects</button>
      <label class="project-name">
        <span>Project name</span>
        <input
          aria-label="Project name"
          maxlength="120"
          value={session.project.name}
          oninput={updateProjectName}
        />
      </label>
      <span class:error-text={saveState === "error"} class="save-state" aria-live="polite">
        {saveState === "saved"
          ? "Saved"
          : saveState === "saving"
            ? "Saving…"
            : saveState === "unsaved"
              ? "Waiting to save"
              : "Save failed"}
      </span>
      <button class="primary-button" type="button" disabled title="Export arrives in M4">
        Export
      </button>
    </header>

    {#if session.sourceStatus !== "ok"}
      <section class:warning={session.sourceStatus === "changed"} class="source-alert" role="alert">
        <div>
          <strong>{sourceStatusLabel}</strong>
          <p>
            {session.sourceStatus === "missing"
              ? "The original video is no longer at its saved location. Relink it to continue."
              : "The source content no longer matches this project. Choose the original or explicitly accept a replacement."}
          </p>
        </div>
        <button type="button" onclick={chooseReplacement} disabled={busyAction === "relinking"}>
          {busyAction === "relinking" ? "Checking…" : "Choose source"}
        </button>
      </section>
    {/if}

    {#if actionError}
      <section class="inline-error" role="alert">
        <div>
          <strong>{actionError.message}</strong>
          {#if actionError.safeDetail}<p>{actionError.safeDetail}</p>{/if}
        </div>
        <code>{actionError.code}</code>
        {#if pendingReplacementPath}
          <button
            type="button"
            onclick={() => applyReplacement(pendingReplacementPath!, true)}
            disabled={busyAction === "relinking"}
          >
            Use changed source
          </button>
        {/if}
      </section>
    {/if}

    <section class="editor-grid">
      <aside class="panel layers-panel">
        <p class="section-label">Project</p>
        <h2>Layers</h2>
        <div class="empty-panel">
          <strong>Source video</strong>
          <span>{session.project.source.filename}</span>
          <small>Overlays become available in M3.</small>
        </div>
      </aside>

      <section class="preview-panel" aria-labelledby="preview-heading">
        <h2 id="preview-heading" class="visually-hidden">Local video preview and crop</h2>
        <PreviewStage
          sourceUrl={previewUrl}
          sourceFilename={session.project.source.filename}
          sourceStatus={session.sourceStatus}
          {sourceSize}
          crop={session.project.crop}
          {playheadMs}
          {playing}
          outMs={session.project.timeline.outMs}
          onCropChange={updateCrop}
          onPlayheadChange={updatePlayhead}
          onPlayingChange={updatePlaying}
          onPreviewError={(message) => (previewError = message)}
        />
        {#if previewError}
          <div class="preview-error" role="alert">{previewError}</div>
        {/if}
        <div class="source-meta">
          <span>{session.project.source.probe.video.displayWidth} × {session.project.source.probe.video.displayHeight}</span>
          <span>{formatDuration(session.project.source.probe.durationMs)}</span>
          <span>{session.project.source.probe.hasAudio ? "Audio detected" : "Silent source"}</span>
        </div>
      </section>

      <aside class="panel inspector-panel">
        <CropInspector
          crop={session.project.crop}
          {sourceSize}
          disabled={session.sourceStatus !== "ok"}
          onChange={updateCrop}
        />
        <dl class="detail-list">
          <div><dt>Codec</dt><dd>{session.project.source.probe.video.codec}</dd></div>
          <div><dt>Container</dt><dd>{session.project.source.probe.containerName}</dd></div>
          <div><dt>Rotation</dt><dd>{session.project.source.probe.video.rotationDegrees}°</dd></div>
          <div>
            <dt>Status</dt>
            <dd>{sourceStatusLabel}</dd>
          </div>
        </dl>
        {#if session.project.source.probe.warnings.length > 0}
          <div class="probe-warnings">
            <strong>Probe notes</strong>
            {#each session.project.source.probe.warnings as warning (warning)}
              <p>{warning}</p>
            {/each}
          </div>
        {/if}
      </aside>
    </section>

    <Timeline
      durationMs={session.project.source.probe.durationMs}
      inMs={session.project.timeline.inMs}
      outMs={session.project.timeline.outMs}
      {playheadMs}
      {playing}
      disabled={session.sourceStatus !== "ok"}
      onInChange={updateTrimIn}
      onOutChange={updateTrimOut}
      onPlayheadChange={updatePlayhead}
      onPlayingChange={updatePlaying}
    />

    <footer class="editor-footer">
      <span>Milestone 2 · Preview, trim, locked crop</span>
      <span>Local project · schema v{session.project.schemaVersion}</span>
    </footer>
  </main>
{:else}
  <main class="home-shell">
    <header class="masthead">
      <div class="brand" aria-label="Skull’d Clip Forge">
        <span class="brand-mark" aria-hidden="true">SCF</span>
        <span><strong>Skull’d</strong><small>Clip Forge</small></span>
      </div>
      <span class="local-badge">Local only</span>
    </header>

    <section class="home-hero" aria-labelledby="page-title">
      <div>
        <p class="eyebrow">Gameplay in. Vertical clip out.</p>
        <h1 id="page-title">Forge the moment.</h1>
        <p class="intro">
          Turn one local gameplay clip into a focused 9:16 project. Your media
          stays on this device.
        </p>
      </div>
      <div
        class:drop-active={dropActive}
        class:busy={busyAction === "importing"}
        class="drop-zone"
        aria-live="polite"
      >
        <span class="drop-icon" aria-hidden="true">↓</span>
        <strong>
          {dropActive
            ? "Release to open clip"
            : busyAction === "importing"
              ? "Reading clip details…"
              : "Drop a gameplay clip here"}
        </strong>
        <span>MP4, MOV, MKV, WebM, M4V, or AVI</span>
        <div class="home-actions">
          <button
            class="primary-button"
            type="button"
            onclick={chooseClip}
            disabled={busyAction !== null}
          >
            Open clip
          </button>
          <button
            class="secondary-button"
            type="button"
            onclick={chooseProject}
            disabled={busyAction !== null}
          >
            Open project
          </button>
        </div>
      </div>
    </section>

    {#if actionError}
      <section class="inline-error home-error" role="alert">
        <div>
          <strong>{actionError.message}</strong>
          {#if actionError.safeDetail}<p>{actionError.safeDetail}</p>{/if}
        </div>
        <code>{actionError.code}</code>
      </section>
    {/if}

    <section class="recents-section" aria-labelledby="recents-heading">
      <div class="section-heading">
        <div>
          <p class="section-label">Continue locally</p>
          <h2 id="recents-heading">Recent projects</h2>
        </div>
        <button class="text-button" type="button" onclick={refreshRecents}>Refresh</button>
      </div>

      {#if recentsLoading}
        <div class="loading-state"><span class="spinner" aria-hidden="true"></span>Loading projects…</div>
      {:else if recents.length === 0}
        <div class="empty-recents">
          <strong>No projects yet</strong>
          <p>Open a gameplay clip to create the first local project.</p>
        </div>
      {:else}
        <div class="recent-grid">
          {#each recents as recent (recent.projectPath)}
            <article class="recent-card">
              <div class="recent-status" data-status={recent.sourceStatus}>
                {recent.sourceStatus === "ok" ? "Ready" : recent.sourceStatus}
              </div>
              <h3>{recent.name}</h3>
              <p>{recent.sourceFilename}</p>
              <div class="recent-meta">
                <span>{formatDuration(recent.durationMs)}</span>
                <span>{formatDate(recent.lastOpenedAt)}</span>
              </div>
              <div class="recent-actions">
                <button type="button" onclick={() => openProject(recent.projectPath)}>Open</button>
                <button type="button" onclick={() => removeRecent(recent.projectPath)}>
                  Remove from recents
                </button>
              </div>
            </article>
          {/each}
        </div>
      {/if}
    </section>

    <footer>
      <span>
        {runtime
          ? `${runtime.os} · ffmpeg ${runtime.ffmpegVersion} · ffprobe ${runtime.ffprobeVersion}`
          : runtimeError
            ? "Media runtime needs attention"
            : "Checking local media runtime…"}
      </span>
      <span>Offline by design</span>
    </footer>
  </main>
{/if}
