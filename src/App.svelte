<script lang="ts">
  import { onMount } from "svelte";

  import type {
    AppError,
    CaptionStyle,
    ExportRequest,
    ExportSettings,
    LoadProjectResult,
    NormalizedRect,
    Overlay,
    RecentProject,
  } from "../contracts/types";
  import CropInspector from "./components/editor/CropInspector.svelte";
  import LayerPanel from "./components/editor/LayerPanel.svelte";
  import OverlayInspector from "./components/editor/OverlayInspector.svelte";
  import PreviewStage from "./components/editor/PreviewStage.svelte";
  import Timeline from "./components/editor/Timeline.svelte";
  import ExportPanel from "./components/export/ExportPanel.svelte";
  import PerformancePanel from "./components/performance/PerformancePanel.svelte";
  import type { RuntimeInfo } from "./contracts/runtime";
  import { createAutosaveScheduler, type AutosaveScheduler } from "./services/autosave";
  import { renderCaption } from "./services/caption-renderer";
  import {
    createExportState,
    exportIsActive,
    reduceExportEvent,
    type ExportState,
  } from "./services/export-state";
  import {
    createCaptionOverlay,
    createImageOverlay,
    createStingOverlay,
    DEFAULT_CAPTION_STYLE,
    insertStingOverlayAtPlayhead,
    MAX_STING_OVERLAYS,
    maximumStingDurationMs,
    nextZIndex,
    reorderOverlay,
    replaceOverlayAsset,
    replaceStingAsset,
    STING_ENTRY_MS,
    STING_EXIT_MS,
  } from "./services/overlay-model";
  import {
    cancelExport,
    createDiagnosticBundle,
    createProject,
    getRuntimeInfo,
    importOverlayAsset,
    importStingAsset,
    listRecentProjects,
    listenForExportEvents,
    listenForFileDrops,
    loadProject,
    mediaPreviewUrl,
    normalizeAppError,
    relinkSource,
    removeRecentProject,
    revealInFolder,
    saveProject,
    selectExportDestination,
    selectDiagnosticDestination,
    selectMediaFile,
    selectOverlayFile,
    selectStingFile,
    selectProjectFile,
    startExport,
    validateExport,
    writeCaptionAsset,
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
  type CaptionStatus = "idle" | "rendering" | "error";

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
  let selectedOverlayId = $state<string | null>(null);
  let overlayBusy = $state(false);
  let captionStatus = $state<CaptionStatus>("idle");
  let exportOpen = $state(false);
  let exportDestination = $state("");
  let exportOverwriteConfirmed = $state(false);
  let exportState = $state<ExportState>(createExportState());
  let diagnosticBusy = $state(false);
  let diagnosticPath = $state<string | null>(null);
  let diagnosticError = $state<string | null>(null);
  let performanceOpen = $state(false);

  let autosave: AutosaveScheduler | null = null;
  let unlistenDrop: (() => void) | null = null;
  let unlistenExport: (() => void) | null = null;
  let captionRenderTimer: ReturnType<typeof setTimeout> | null = null;
  let captionRenderPromise: Promise<void> | null = null;
  let captionRenderError: AppError | null = null;
  let pendingCaptionId: string | null = null;
  let captionRevision = 0;

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
  const selectedOverlay = $derived(
    session?.project.overlays.find(({ id }) => id === selectedOverlayId) ?? null,
  );

  onMount(() => {
    void bootstrap();
    return () => {
      unlistenDrop?.();
      unlistenExport?.();
      autosave?.dispose();
      if (captionRenderTimer) {
        clearTimeout(captionRenderTimer);
      }
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
      [unlistenDrop, unlistenExport] = await Promise.all([
        listenForFileDrops(
          (paths) => {
            if (!session && paths[0]) {
              void importClip(paths[0]);
            }
          },
          (active) => {
            dropActive = active;
          },
        ),
        listenForExportEvents((event) => {
          exportState = reduceExportEvent(exportState, event);
        }),
      ]);
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
    cancelPendingCaptionRender();
    session = next;
    pendingReplacementPath = null;
    playheadMs = next.project.timeline.inMs;
    playing = false;
    previewError = null;
    selectedOverlayId = null;
    overlayBusy = false;
    captionStatus = "idle";
    captionRenderError = null;
    saveState = "saved";
    exportOpen = false;
    exportDestination = "";
    exportOverwriteConfirmed = false;
    exportState = createExportState();
    diagnosticBusy = false;
    diagnosticPath = null;
    diagnosticError = null;
    performanceOpen = false;
    autosave = createAutosaveScheduler(persistProject);
  }

  function openExport(): void {
    if (!session || session.sourceStatus !== "ok") {
      return;
    }
    if (
      exportState.status === "completed" ||
      exportState.status === "cancelled" ||
      exportState.status === "error"
    ) {
      exportState = createExportState();
    }
    playing = false;
    exportOpen = true;
  }

  function closeExport(): void {
    if (exportIsActive(exportState)) {
      return;
    }
    exportOpen = false;
  }

  function openPerformance(): void {
    playing = false;
    performanceOpen = true;
  }

  function closePerformance(): void {
    performanceOpen = false;
  }

  function updateExportSettings(settings: ExportSettings): void {
    if (!session || exportIsActive(exportState)) {
      return;
    }
    session.project.exportDefaults = settings;
    exportState = createExportState();
    exportOverwriteConfirmed = false;
    markProjectDirty();
  }

  async function chooseExportDestination(): Promise<void> {
    if (!session || exportIsActive(exportState)) {
      return;
    }
    try {
      const destination = await selectExportDestination(
        `${session.project.name}.mp4`,
      );
      if (destination) {
        exportDestination = destination;
        exportOverwriteConfirmed = false;
        exportState = createExportState();
        diagnosticPath = null;
        diagnosticError = null;
      }
    } catch (error) {
      exportState = {
        ...createExportState(),
        status: "error",
        error: normalizeAppError(error),
      };
    }
  }

  function createExportRequest(): ExportRequest | null {
    if (!session || !exportDestination) {
      return null;
    }
    return {
      projectPath: session.projectPath,
      projectSnapshot: $state.snapshot(session.project),
      destinationPath: exportDestination,
      overwrite: exportOverwriteConfirmed,
      settings: $state.snapshot(session.project.exportDefaults),
    };
  }

  async function startCurrentExport(): Promise<void> {
    if (!session || exportIsActive(exportState)) {
      return;
    }
    if (!exportDestination) {
      await chooseExportDestination();
      return;
    }
    exportState = {
      ...createExportState(),
      status: "validating",
    };
    try {
      await flushPendingCaptionRender();
      await autosave?.flush();
      if (saveState === "error") {
        throw actionError ?? {
          code: "E_IO",
          message: "The project could not be saved before export.",
          safeDetail: "Resolve the save error, then retry export.",
          retryable: true,
        };
      }
      const request = createExportRequest();
      if (!request) {
        return;
      }
      const validation = await validateExport(request);
      if (!validation.valid) {
        exportState = {
          ...createExportState(),
          validation,
        };
        return;
      }
      exportState = {
        ...createExportState(),
        status: "starting",
        validation,
      };
      const started = await startExport(request);
      if (exportState.status === "starting" && exportState.jobId === null) {
        exportState = {
          ...exportState,
          status: "running",
          jobId: started.jobId,
        };
      }
    } catch (error) {
      exportState = {
        ...exportState,
        status: "error",
        phase: null,
        error: normalizeAppError(error),
      };
    }
  }

  async function cancelCurrentExport(): Promise<void> {
    const jobId = exportState.jobId;
    if (!jobId || !exportIsActive(exportState)) {
      return;
    }
    exportState = { ...exportState, cancelRequested: true };
    try {
      await cancelExport(jobId);
    } catch (error) {
      exportState = {
        ...exportState,
        status: "error",
        error: normalizeAppError(error),
        cancelRequested: false,
      };
    }
  }

  async function createExportDiagnostic(): Promise<void> {
    if (!session || diagnosticBusy) {
      return;
    }
    diagnosticBusy = true;
    diagnosticError = null;
    try {
      const destination = await selectDiagnosticDestination(
        `${session.project.name}-diagnostics.zip`,
      );
      if (!destination) {
        return;
      }
      const result = await createDiagnosticBundle(
        destination,
        session.projectPath,
      );
      diagnosticPath = result.path;
    } catch (error) {
      const normalized = normalizeAppError(error);
      diagnosticError = normalized.safeDetail
        ? `${normalized.message} ${normalized.safeDetail}`
        : normalized.message;
    } finally {
      diagnosticBusy = false;
    }
  }

  async function revealExportOutput(): Promise<void> {
    if (!exportState.outputPath) {
      return;
    }
    diagnosticError = null;
    try {
      await revealInFolder(exportState.outputPath);
    } catch (error) {
      const normalized = normalizeAppError(error);
      diagnosticError = normalized.safeDetail
        ? `${normalized.message} ${normalized.safeDetail}`
        : normalized.message;
    }
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

  function updateOverlay(overlay: Overlay): void {
    if (!session || session.sourceStatus !== "ok") {
      return;
    }
    const index = session.project.overlays.findIndex(({ id }) => id === overlay.id);
    if (index < 0) {
      return;
    }
    session.project.overlays[index] = overlay;
    markProjectDirty();
  }

  async function addImageOverlay(): Promise<void> {
    if (!session || session.sourceStatus !== "ok" || overlayBusy) {
      return;
    }
    if (session.project.overlays.length >= 100) {
      actionError = overlayLimitError();
      return;
    }
    actionError = null;
    try {
      const sourceAssetPath = await selectOverlayFile();
      if (!sourceAssetPath || !session) {
        return;
      }
      overlayBusy = true;
      const asset = await importOverlayAsset(session.projectPath, sourceAssetPath);
      const overlay = createImageOverlay(
        crypto.randomUUID(),
        asset,
        session.project.timeline.inMs,
        session.project.timeline.outMs,
        nextZIndex(session.project.overlays),
      );
      session.project.overlays.push(overlay);
      selectOverlay(overlay.id);
      markProjectDirty();
    } catch (error) {
      actionError = normalizeAppError(error);
    } finally {
      overlayBusy = false;
    }
  }

  async function addCaptionOverlay(text: string): Promise<boolean> {
    if (!session || session.sourceStatus !== "ok" || overlayBusy) {
      return false;
    }
    if (session.project.overlays.length >= 100) {
      actionError = overlayLimitError();
      return false;
    }
    overlayBusy = true;
    captionStatus = "rendering";
    actionError = null;
    try {
      const caption = { ...DEFAULT_CAPTION_STYLE, text: text.trim() };
      const rendered = await renderCaption(caption);
      if (!session) {
        return false;
      }
      const asset = await writeCaptionAsset(
        session.projectPath,
        rendered.contentHash,
        rendered.pngBytesBase64,
        rendered.width,
        rendered.height,
      );
      const overlay = createCaptionOverlay(
        crypto.randomUUID(),
        caption.text,
        caption,
        asset,
        session.project.timeline.inMs,
        session.project.timeline.outMs,
        nextZIndex(session.project.overlays),
      );
      session.project.overlays.push(overlay);
      selectOverlay(overlay.id);
      captionStatus = "idle";
      markProjectDirty();
      return true;
    } catch (error) {
      captionStatus = "error";
      actionError = normalizeCaptionRenderError(error);
      return false;
    } finally {
      overlayBusy = false;
    }
  }

  async function addStingOverlay(insertAtMs?: number): Promise<void> {
    if (!session || session.sourceStatus !== "ok" || overlayBusy) {
      return;
    }
    const stingCount = session.project.overlays.filter(
      (overlay) => overlay.type === "sting",
    ).length;
    if (stingCount >= MAX_STING_OVERLAYS) {
      actionError = {
        code: "E_INVALID_ARGUMENT",
        message: "This project has reached the eight-sting limit.",
        safeDetail: "Remove an existing sting before adding another.",
        retryable: false,
      };
      return;
    }
    actionError = null;
    try {
      const sourceAssetPath = await selectStingFile();
      if (!sourceAssetPath || !session) {
        return;
      }
      overlayBusy = true;
      const asset = await importStingAsset(session.projectPath, sourceAssetPath);
      let overlay = createStingOverlay(
        crypto.randomUUID(),
        asset,
        session.project.timeline.inMs,
        session.project.timeline.outMs,
        nextZIndex(session.project.overlays),
      );
      if (insertAtMs !== undefined) {
        overlay = insertStingOverlayAtPlayhead(
          overlay,
          overlay.id,
          insertAtMs,
          session.project.timeline.inMs,
          session.project.timeline.outMs,
          overlay.zIndex,
        );
      }
      session.project.overlays.push(overlay);
      if (insertAtMs === undefined) {
        selectOverlay(overlay.id);
      } else {
        selectedOverlayId = overlay.id;
        captionStatus = "idle";
      }
      markProjectDirty();
    } catch (error) {
      actionError =
        error instanceof Error
          ? {
              code: "E_INVALID_ARGUMENT",
              message: "The Skull'd sting could not be added.",
              safeDetail: error.message,
              retryable: false,
            }
          : normalizeAppError(error);
    } finally {
      overlayBusy = false;
    }
  }

  async function insertStingAtPlayhead(): Promise<void> {
    if (!session || session.sourceStatus !== "ok" || overlayBusy) {
      return;
    }
    const stings = session.project.overlays.filter(
      (overlay) => overlay.type === "sting",
    );
    if (stings.length >= MAX_STING_OVERLAYS) {
      actionError = {
        code: "E_INVALID_ARGUMENT",
        message: "This project has reached the eight-sting limit.",
        safeDetail: "Remove an existing sting before inserting another.",
        retryable: false,
      };
      return;
    }
    const template =
      stings.find(({ id }) => id === selectedOverlayId) ??
      [...stings].sort((a, b) => b.zIndex - a.zIndex)[0];
    if (!template) {
      await addStingOverlay(playheadMs);
      return;
    }
    const inserted = insertStingOverlayAtPlayhead(
      template,
      crypto.randomUUID(),
      playheadMs,
      session.project.timeline.inMs,
      session.project.timeline.outMs,
      nextZIndex(session.project.overlays),
    );
    session.project.overlays.push(inserted);
    selectedOverlayId = inserted.id;
    captionStatus = "idle";
    actionError = null;
    markProjectDirty();
  }

  async function replaceImageOverlay(id: string): Promise<void> {
    if (!session || session.sourceStatus !== "ok" || overlayBusy) {
      return;
    }
    actionError = null;
    try {
      const sourceAssetPath = await selectOverlayFile();
      if (!sourceAssetPath || !session) {
        return;
      }
      overlayBusy = true;
      const asset = await importOverlayAsset(session.projectPath, sourceAssetPath);
      const overlay = session.project.overlays.find(({ id: candidate }) => candidate === id);
      if (!overlay || overlay.type !== "image") {
        return;
      }
      updateOverlay(replaceOverlayAsset(overlay, asset));
    } catch (error) {
      actionError = normalizeAppError(error);
    } finally {
      overlayBusy = false;
    }
  }

  async function replaceStingOverlay(id: string): Promise<void> {
    if (!session || session.sourceStatus !== "ok" || overlayBusy) {
      return;
    }
    actionError = null;
    try {
      const sourceAssetPath = await selectStingFile();
      if (!sourceAssetPath || !session) {
        return;
      }
      overlayBusy = true;
      const asset = await importStingAsset(session.projectPath, sourceAssetPath);
      const overlay = session.project.overlays.find(({ id: candidate }) => candidate === id);
      if (!overlay || overlay.type !== "sting") {
        return;
      }
      updateOverlay(replaceStingAsset(overlay, asset));
    } catch (error) {
      actionError = normalizeAppError(error);
    } finally {
      overlayBusy = false;
    }
  }

  function duplicateStingOverlay(id: string): void {
    if (!session) {
      return;
    }
    const stingCount = session.project.overlays.filter(
      (overlay) => overlay.type === "sting",
    ).length;
    if (stingCount >= MAX_STING_OVERLAYS) {
      actionError = {
        code: "E_INVALID_ARGUMENT",
        message: "This project has reached the eight-sting limit.",
        safeDetail: "Remove an existing sting before duplicating it.",
        retryable: false,
      };
      return;
    }
    const source = session.project.overlays.find(
      (overlay) => overlay.id === id && overlay.type === "sting",
    );
    if (!source || source.type !== "sting") {
      return;
    }
    const { inMs, outMs } = session.project.timeline;
    const durationMs = source.endMs - source.startMs;
    const startMs = Math.max(
      inMs,
      Math.min(source.endMs + 250, outMs - durationMs),
    );
    const duplicate: typeof source = {
      ...source,
      id: crypto.randomUUID(),
      name: `${source.name.slice(0, 115)} copy`,
      position: { ...source.position },
      startMs,
      endMs: startMs + durationMs,
      zIndex: nextZIndex(session.project.overlays),
    };
    session.project.overlays.push(duplicate);
    selectOverlay(duplicate.id);
    actionError = null;
    markProjectDirty();
  }

  function deleteOverlay(id: string): void {
    if (!session) {
      return;
    }
    session.project.overlays = session.project.overlays.filter(
      ({ id: candidate }) => candidate !== id,
    );
    if (pendingCaptionId === id) {
      cancelPendingCaptionRender();
    }
    if (selectedOverlayId === id) {
      selectOverlay(null);
    }
    markProjectDirty();
  }

  function selectOverlay(id: string | null): void {
    selectedOverlayId = id;
    const overlay = session?.project.overlays.find(
      ({ id: candidate }) => candidate === id,
    );
    if (overlay?.type === "sting" && session) {
      const visibleStart = Math.min(
        overlay.endMs,
        overlay.startMs + STING_ENTRY_MS,
      );
      const visibleEnd = Math.max(
        visibleStart,
        overlay.endMs - STING_EXIT_MS,
      );
      if (playheadMs < visibleStart || playheadMs > visibleEnd) {
        updatePlayhead(
          Math.round(visibleStart + (visibleEnd - visibleStart) / 2),
        );
      }
    }
    if (!overlay || overlay.type !== "caption") {
      captionStatus = "idle";
    }
  }

  function moveOverlayInStack(id: string, direction: -1 | 1): void {
    if (!session) {
      return;
    }
    session.project.overlays = reorderOverlay(
      session.project.overlays,
      id,
      direction,
    );
    markProjectDirty();
  }

  function updateCaptionOverlay(id: string, caption: CaptionStyle): void {
    if (!session) {
      return;
    }
    const overlay = session.project.overlays.find(
      ({ id: candidate }) => candidate === id,
    );
    if (!overlay || overlay.type !== "caption") {
      return;
    }
    overlay.caption = caption;
    captionRevision += 1;
    captionRenderError = null;
    captionStatus = "rendering";
    if (captionRenderTimer) {
      clearTimeout(captionRenderTimer);
    }
    const revision = captionRevision;
    pendingCaptionId = id;
    captionRenderTimer = setTimeout(() => {
      captionRenderTimer = null;
      pendingCaptionId = null;
      startCaptionRender(id, revision);
    }, 250);
    markProjectDirty();
  }

  function startCaptionRender(id: string, revision: number): Promise<void> {
    const promise = renderCaptionRevision(id, revision);
    captionRenderPromise = promise;
    void promise.then(() => {
      if (captionRenderPromise === promise) {
        captionRenderPromise = null;
      }
    });
    return promise;
  }

  async function renderCaptionRevision(
    id: string,
    revision: number,
  ): Promise<void> {
    const active = session;
    const overlay = active?.project.overlays.find(
      ({ id: candidate }) => candidate === id,
    );
    if (!active || !overlay || overlay.type !== "caption") {
      return;
    }
    const caption = { ...overlay.caption };
    try {
      const rendered = await renderCaption(caption);
      const asset = await writeCaptionAsset(
        active.projectPath,
        rendered.contentHash,
        rendered.pngBytesBase64,
        rendered.width,
        rendered.height,
      );
      if (
        session?.projectPath !== active.projectPath ||
        revision !== captionRevision
      ) {
        return;
      }
      const current = session.project.overlays.find(
        ({ id: candidate }) => candidate === id,
      );
      if (!current || current.type !== "caption") {
        return;
      }
      const updated = replaceOverlayAsset(current, asset);
      current.generatedAsset = asset;
      current.position = updated.position;
      captionStatus = "idle";
      captionRenderError = null;
    } catch (error) {
      if (revision === captionRevision) {
        captionRenderError = normalizeCaptionRenderError(error);
        captionStatus = "error";
        actionError = captionRenderError;
      }
    }
  }

  async function flushPendingCaptionRender(): Promise<void> {
    if (captionRenderTimer && pendingCaptionId) {
      const id = pendingCaptionId;
      clearTimeout(captionRenderTimer);
      captionRenderTimer = null;
      pendingCaptionId = null;
      await startCaptionRender(id, captionRevision);
    } else if (captionRenderPromise) {
      await captionRenderPromise;
    }
    if (captionRenderError) {
      throw captionRenderError;
    }
  }

  function cancelPendingCaptionRender(): void {
    if (captionRenderTimer) {
      clearTimeout(captionRenderTimer);
      captionRenderTimer = null;
    }
    pendingCaptionId = null;
    captionRevision += 1;
    captionRenderPromise = null;
    captionRenderError = null;
  }

  function clampOverlayTimings(): void {
    if (!session) {
      return;
    }
    const { inMs, outMs } = session.project.timeline;
    for (const overlay of session.project.overlays) {
      const minimumDuration = overlay.type === "sting" ? 500 : 1;
      const start = Math.max(
        inMs,
        Math.min(overlay.startMs, outMs - minimumDuration),
      );
      const maximumEnd =
        overlay.type === "sting"
          ? start +
            maximumStingDurationMs(
              { ...overlay, startMs: start },
              outMs,
            )
          : outMs;
      const end = Math.min(
        maximumEnd,
        Math.max(overlay.endMs, start + minimumDuration),
      );
      overlay.startMs = start;
      overlay.endMs = end;
    }
  }

  function updateTrimIn(requestedMs: number): void {
    if (!session) {
      return;
    }
    const timeline = session.project.timeline;
    const minimumDuration = session.project.overlays.some(
      (overlay) => overlay.type === "sting",
    )
      ? 500
      : 250;
    timeline.inMs = setTrimIn(
      Math.min(requestedMs, timeline.outMs - minimumDuration),
      timeline.outMs,
      session.project.source.probe.durationMs,
    );
    clampOverlayTimings();
    playheadMs = clampPlayhead(playheadMs, timeline.inMs, timeline.outMs);
    markProjectDirty();
  }

  function updateTrimOut(requestedMs: number): void {
    if (!session) {
      return;
    }
    const timeline = session.project.timeline;
    const minimumDuration = session.project.overlays.some(
      (overlay) => overlay.type === "sting",
    )
      ? 500
      : 250;
    timeline.outMs = setTrimOut(
      Math.max(requestedMs, timeline.inMs + minimumDuration),
      timeline.inMs,
      session.project.source.probe.durationMs,
    );
    clampOverlayTimings();
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
      await flushPendingCaptionRender();
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
    if (exportIsActive(exportState)) {
      exportOpen = true;
      return;
    }
    playing = false;
    await autosave?.flush();
    if (saveState === "error") {
      return;
    }
    autosave?.dispose();
    autosave = null;
    cancelPendingCaptionRender();
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

  function normalizeCaptionRenderError(error: unknown): AppError {
    if (error instanceof Error) {
      return {
        code: "E_INVALID_ARGUMENT",
        message: "The caption could not be rendered.",
        safeDetail: error.message,
        retryable: false,
      };
    }
    return normalizeAppError(error);
  }

  function overlayLimitError(): AppError {
    return {
      code: "E_INVALID_ARGUMENT",
      message: "The project overlay limit has been reached.",
      safeDetail: "Delete an existing overlay before adding another.",
      retryable: false,
    };
  }

  function handleShortcut(event: KeyboardEvent): void {
    if (performanceOpen) {
      if (event.key === "Escape") {
        event.preventDefault();
        closePerformance();
      }
      return;
    }
    if (!session || (isTypingTarget(event.target) && event.key !== "Escape")) {
      return;
    }
    const timeline = session.project.timeline;
    const modifier = event.metaKey || event.ctrlKey;
    if (exportOpen) {
      if (event.key === "Escape" && !exportIsActive(exportState)) {
        event.preventDefault();
        closeExport();
      }
      return;
    }
    if (modifier && event.key.toLowerCase() === "s") {
      event.preventDefault();
      void autosave?.flush();
      return;
    }
    if (modifier && event.key.toLowerCase() === "e") {
      event.preventDefault();
      openExport();
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
      case "Delete":
      case "Backspace":
        if (selectedOverlayId) {
          event.preventDefault();
          deleteOverlay(selectedOverlayId);
        }
        break;
      case "Escape":
        selectOverlay(null);
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
      <button
        class="secondary-button compact-button"
        type="button"
        onclick={openPerformance}
      >
        Performance
      </button>
      <button
        class="primary-button"
        type="button"
        onclick={openExport}
        disabled={session.sourceStatus !== "ok"}
        title="Export vertical MP4 (Ctrl/Cmd+E)"
      >
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
        <LayerPanel
          overlays={session.project.overlays}
          sourceFilename={session.project.source.filename}
          {selectedOverlayId}
          busy={overlayBusy || session.sourceStatus !== "ok"}
          onSelect={selectOverlay}
          onAddImage={addImageOverlay}
          onAddSting={addStingOverlay}
          onAddCaption={addCaptionOverlay}
        />
      </aside>

      <section class="preview-panel" aria-labelledby="preview-heading">
        <h2 id="preview-heading" class="visually-hidden">Local video preview and crop</h2>
        <PreviewStage
          sourceUrl={previewUrl}
          sourceFilename={session.project.source.filename}
          sourceStatus={session.sourceStatus}
          {sourceSize}
          crop={session.project.crop}
          overlays={session.project.overlays}
          projectPath={session.projectPath}
          {selectedOverlayId}
          {playheadMs}
          {playing}
          outMs={session.project.timeline.outMs}
          onCropChange={updateCrop}
          onOverlayChange={updateOverlay}
          onOverlaySelect={selectOverlay}
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
        {#if selectedOverlay}
          <OverlayInspector
            overlay={selectedOverlay}
            timelineInMs={session.project.timeline.inMs}
            timelineOutMs={session.project.timeline.outMs}
            {playheadMs}
            {captionStatus}
            disabled={overlayBusy || session.sourceStatus !== "ok"}
            onChange={updateOverlay}
            onCaptionChange={updateCaptionOverlay}
            onReplaceImage={replaceImageOverlay}
            onReplaceSting={replaceStingOverlay}
            onDuplicateSting={duplicateStingOverlay}
            onReorder={moveOverlayInStack}
            onDelete={deleteOverlay}
          />
        {:else}
          <CropInspector
            crop={session.project.crop}
            {sourceSize}
            disabled={session.sourceStatus !== "ok"}
            onChange={updateCrop}
          />
        {/if}
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

      <Timeline
        durationMs={session.project.source.probe.durationMs}
        inMs={session.project.timeline.inMs}
        outMs={session.project.timeline.outMs}
        {playheadMs}
        {playing}
        overlays={session.project.overlays}
        disabled={session.sourceStatus !== "ok"}
        insertStingDisabled={overlayBusy ||
          session.project.overlays.filter((overlay) => overlay.type === "sting").length >=
            MAX_STING_OVERLAYS}
        onInChange={updateTrimIn}
        onOutChange={updateTrimOut}
        onInsertSting={insertStingAtPlayhead}
        onPlayheadChange={updatePlayhead}
        onPlayingChange={updatePlaying}
      />
    </section>

    <footer class="editor-footer">
      <span>Milestone 7 · YouTube performance</span>
      <span>Local project · schema v{session.project.schemaVersion}</span>
    </footer>

    {#if exportOpen}
      <ExportPanel
        projectName={session.project.name}
        settings={session.project.exportDefaults}
        sourceHasAudio={session.project.source.probe.hasAudio}
        destinationPath={exportDestination}
        overwriteConfirmed={exportOverwriteConfirmed}
        state={exportState}
        {diagnosticBusy}
        {diagnosticPath}
        {diagnosticError}
        onSettingsChange={updateExportSettings}
        onChooseDestination={chooseExportDestination}
        onOverwriteChange={(confirmed) => (exportOverwriteConfirmed = confirmed)}
        onStart={startCurrentExport}
        onCancel={cancelCurrentExport}
        onCreateDiagnostic={createExportDiagnostic}
        onReveal={revealExportOutput}
        onClose={closeExport}
      />
    {/if}
  </main>
{:else}
  <main class="home-shell">
    <header class="masthead">
      <div class="brand" aria-label="Skull’d Clip Forge">
        <span class="brand-mark" aria-hidden="true">SCF</span>
        <span><strong>Skull’d</strong><small>Clip Forge</small></span>
      </div>
      <div class="masthead-actions">
        <span class="local-badge">Editor works offline</span>
        <button class="secondary-button compact-button" type="button" onclick={openPerformance}>
          Channel performance
        </button>
      </div>
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

{#if performanceOpen}
  <PerformancePanel
    project={session ? { id: session.project.id, name: session.project.name } : null}
    onClose={closePerformance}
  />
{/if}
