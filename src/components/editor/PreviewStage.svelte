<script lang="ts">
  import { onMount } from "svelte";
  import StingPreview from "./StingPreview.svelte";

  import type {
    AssetRef,
    NormalizedRect,
    Overlay,
    SourceStatus,
  } from "../../../contracts/types";
  import {
    fitSourceInsideStage,
    normalizedRectToStage,
    stagePointToNormalized,
    type Point,
    type Size,
  } from "../../services/coordinate-mapper";
  import {
    moveCrop,
    nudgeCrop,
    resetCrop,
    zoomCrop,
  } from "../../services/crop-solver";
  import {
    isOverlayVisible,
    moveOverlay,
    overlayAsset,
    resizeOverlay,
    stingDisplayX,
    stingPlaybackRate,
    stingRepeats,
  } from "../../services/overlay-model";
  import { projectAssetPreviewUrl } from "../../services/tauri";

  interface Props {
    sourceUrl: string | null;
    sourceFilename: string;
    sourceStatus: SourceStatus;
    sourceSize: Size;
    crop: NormalizedRect;
    overlays: Overlay[];
    projectPath: string;
    selectedOverlayId: string | null;
    playheadMs: number;
    playing: boolean;
    outMs: number;
    onCropChange: (crop: NormalizedRect) => void;
    onOverlayChange: (overlay: Overlay) => void;
    onOverlaySelect: (id: string | null) => void;
    onPlayheadChange: (milliseconds: number) => void;
    onPlayingChange: (playing: boolean) => void;
    onPreviewError: (message: string | null) => void;
  }

  let {
    sourceUrl,
    sourceFilename,
    sourceStatus,
    sourceSize,
    crop,
    overlays,
    projectPath,
    selectedOverlayId,
    playheadMs,
    playing,
    outMs,
    onCropChange,
    onOverlayChange,
    onOverlaySelect,
    onPlayheadChange,
    onPlayingChange,
    onPreviewError,
  }: Props = $props();

  let stageElement = $state<HTMLDivElement | null>(null);
  let videoElement = $state<HTMLVideoElement | null>(null);
  let stageSize = $state<Size>({ width: 1, height: 1 });
  let drag:
    | {
        mode: "pan" | "resize";
        pointer: Point;
        crop: NormalizedRect;
        captureTarget: HTMLElement;
      }
    | null = null;
  let overlayDrag:
    | {
        id: string;
        mode: "pan" | "resize";
        pointer: Point;
        position: NormalizedRect;
        asset: AssetRef;
        captureTarget: HTMLElement;
      }
    | null = null;

  const cropPixels = $derived(
    normalizedRectToStage(crop, sourceSize, stageSize),
  );
  const visibleOverlays = $derived(
    [...overlays]
      .filter((overlay) => isOverlayVisible(overlay, playheadMs))
      .sort((a, b) => a.zIndex - b.zIndex),
  );

  onMount(() => {
    if (!stageElement) {
      return;
    }
    const observer = new ResizeObserver(([entry]) => {
      if (entry) {
        stageSize = {
          width: Math.max(1, entry.contentRect.width),
          height: Math.max(1, entry.contentRect.height),
        };
      }
    });
    observer.observe(stageElement);
    return () => observer.disconnect();
  });

  $effect(() => {
    const video = videoElement;
    if (!video || sourceStatus !== "ok") {
      return;
    }
    if (!playing && Math.abs(video.currentTime * 1_000 - playheadMs) > 25) {
      video.currentTime = playheadMs / 1_000;
    }
  });

  $effect(() => {
    const video = videoElement;
    if (!video || sourceStatus !== "ok") {
      return;
    }
    if (playing) {
      void video.play().catch(() => {
        onPlayingChange(false);
        onPreviewError("Playback could not start for this local source.");
      });
    } else {
      video.pause();
    }
  });

  function pointerPoint(event: { clientX: number; clientY: number }): Point {
    const bounds = stageElement?.getBoundingClientRect();
    return {
      x: event.clientX - (bounds?.left ?? 0),
      y: event.clientY - (bounds?.top ?? 0),
    };
  }

  function beginDrag(event: PointerEvent, mode: "pan" | "resize"): void {
    event.preventDefault();
    event.stopPropagation();
    onOverlaySelect(null);
    const captureTarget = event.currentTarget as HTMLElement;
    captureTarget.setPointerCapture(event.pointerId);
    drag = {
      mode,
      pointer: pointerPoint(event),
      crop: { ...crop },
      captureTarget,
    };
  }

  function beginOverlayDrag(
    event: PointerEvent,
    overlay: Overlay,
    mode: "pan" | "resize",
  ): void {
    event.preventDefault();
    event.stopPropagation();
    const captureTarget = event.currentTarget as HTMLElement;
    captureTarget.setPointerCapture(event.pointerId);
    onOverlaySelect(overlay.id);
    overlayDrag = {
      id: overlay.id,
      mode,
      pointer: pointerPoint(event),
      position: { ...overlay.position },
      asset: overlayAsset(overlay),
      captureTarget,
    };
  }

  function continueOverlayDrag(event: PointerEvent): void {
    if (!overlayDrag || cropPixels.width <= 0 || cropPixels.height <= 0) {
      return;
    }
    const overlay = overlays.find(({ id }) => id === overlayDrag?.id);
    if (!overlay) {
      return;
    }
    const current = pointerPoint(event);
    const dx = (current.x - overlayDrag.pointer.x) / cropPixels.width;
    const dy = (current.y - overlayDrag.pointer.y) / cropPixels.height;
    const position =
      overlayDrag.mode === "pan"
        ? moveOverlay(overlayDrag.position, dx, dy)
        : resizeOverlay(
            overlayDrag.position,
            overlayDrag.asset,
            overlayDrag.position.width + Math.max(dx, dy),
          );
    onOverlayChange({ ...overlay, position });
  }

  function endOverlayDrag(event: PointerEvent): void {
    if (!overlayDrag) {
      return;
    }
    if (overlayDrag.captureTarget.hasPointerCapture(event.pointerId)) {
      overlayDrag.captureTarget.releasePointerCapture(event.pointerId);
    }
    overlayDrag = null;
  }

  function handleOverlayKey(event: KeyboardEvent, overlay: Overlay): void {
    const multiplier = event.shiftKey ? 10 : 1;
    const movements: Record<string, Point> = {
      ArrowLeft: { x: -multiplier / 1080, y: 0 },
      ArrowRight: { x: multiplier / 1080, y: 0 },
      ArrowUp: { x: 0, y: -multiplier / 1920 },
      ArrowDown: { x: 0, y: multiplier / 1920 },
    };
    const movement = movements[event.key];
    if (!movement) {
      return;
    }
    event.preventDefault();
    event.stopPropagation();
    onOverlayChange({
      ...overlay,
      position: moveOverlay(overlay.position, movement.x, movement.y),
    });
  }

  function overlayStyle(overlay: Overlay): string {
    const position = overlay.position;
    const displayX =
      overlay.type === "sting" ? stingDisplayX(overlay, playheadMs) : position.x;
    return [
      `left:${cropPixels.x + displayX * cropPixels.width}px`,
      `top:${cropPixels.y + position.y * cropPixels.height}px`,
      `width:${position.width * cropPixels.width}px`,
      `height:${position.height * cropPixels.height}px`,
      `opacity:${overlay.opacity}`,
      `z-index:${20 + overlay.zIndex}`,
    ].join(";");
  }

  function continueDrag(event: PointerEvent): void {
    if (!drag) {
      return;
    }
    const current = pointerPoint(event);
    const video = fitSourceInsideStage(sourceSize, stageSize);
    const dx = (current.x - drag.pointer.x) / video.width;
    const dy = (current.y - drag.pointer.y) / video.height;
    if (drag.mode === "pan") {
      onCropChange(moveCrop(drag.crop, { x: dx, y: dy }));
      return;
    }
    const factor = Math.max(0.05, 1 + dy / drag.crop.height);
    onCropChange(
      zoomCrop(
        drag.crop,
        factor,
        { x: drag.crop.x, y: drag.crop.y },
        sourceSize,
      ),
    );
  }

  function endDrag(event: PointerEvent): void {
    if (drag) {
      if (drag.captureTarget.hasPointerCapture(event.pointerId)) {
        drag.captureTarget.releasePointerCapture(event.pointerId);
      }
      drag = null;
    }
  }

  function zoomAtPointer(event: WheelEvent): void {
    event.preventDefault();
    const anchor = stagePointToNormalized(
      pointerPoint(event),
      sourceSize,
      stageSize,
    );
    onCropChange(zoomCrop(crop, Math.exp(event.deltaY * 0.0015), anchor, sourceSize));
  }

  function handleCropKey(event: KeyboardEvent): void {
    const step = event.shiftKey ? 10 : 1;
    const delta: Record<string, Point> = {
      ArrowLeft: { x: -step, y: 0 },
      ArrowRight: { x: step, y: 0 },
      ArrowUp: { x: 0, y: -step },
      ArrowDown: { x: 0, y: step },
    };
    const movement = delta[event.key];
    if (!movement) {
      return;
    }
    event.preventDefault();
    event.stopPropagation();
    onCropChange(
      nudgeCrop(crop, sourceSize, movement.x, movement.y),
    );
  }

  function handleTimeUpdate(): void {
    if (!videoElement) {
      return;
    }
    const currentMs = Math.round(videoElement.currentTime * 1_000);
    if (currentMs >= outMs) {
      videoElement.pause();
      videoElement.currentTime = outMs / 1_000;
      onPlayheadChange(outMs);
      onPlayingChange(false);
      return;
    }
    onPlayheadChange(currentMs);
  }
</script>

<div
  class="preview-stage"
  bind:this={stageElement}
  style={`aspect-ratio:${sourceSize.width} / ${sourceSize.height}`}
>
  {#if sourceStatus === "ok" && sourceUrl}
    <!-- svelte-ignore a11y_media_has_caption -->
    <video
      bind:this={videoElement}
      src={sourceUrl}
      aria-label={`Local preview of ${sourceFilename}`}
      preload="metadata"
      playsinline
      onloadedmetadata={() => {
        if (videoElement) {
          videoElement.currentTime = playheadMs / 1_000;
        }
        onPreviewError(null);
      }}
      ontimeupdate={handleTimeUpdate}
      onplay={() => onPlayingChange(true)}
      onpause={() => onPlayingChange(false)}
      onerror={() => onPreviewError("The local video preview could not be decoded.")}
    ></video>
    <button
      class="crop-frame"
      type="button"
      aria-label="Locked 9:16 crop. Drag to pan; arrows nudge; Shift plus arrows nudges faster."
      title="Drag to pan · wheel to zoom · double-click to reset"
      style={`left:${cropPixels.x}px;top:${cropPixels.y}px;width:${cropPixels.width}px;height:${cropPixels.height}px`}
      onpointerdown={(event) => beginDrag(event, "pan")}
      onpointermove={continueDrag}
      onpointerup={endDrag}
      onpointercancel={endDrag}
      onwheel={zoomAtPointer}
      ondblclick={() => onCropChange(resetCrop(sourceSize))}
      onkeydown={handleCropKey}
    >
      <span class="crop-label">9:16 crop</span>
      <span class="safe-area-guide" aria-hidden="true"></span>
      <span
        class="resize-handle"
        role="presentation"
        onpointerdown={(event) => beginDrag(event, "resize")}
      ></span>
    </button>
    {#each visibleOverlays as overlay (overlay.id)}
      <button
        class:selected={selectedOverlayId === overlay.id}
        class="overlay-frame"
        type="button"
        aria-label={`${overlay.name} ${overlay.type} overlay. Drag to move; arrows nudge.`}
        style={overlayStyle(overlay)}
        onpointerdown={(event) => beginOverlayDrag(event, overlay, "pan")}
        onpointermove={continueOverlayDrag}
        onpointerup={endOverlayDrag}
        onpointercancel={endOverlayDrag}
        onkeydown={(event) => handleOverlayKey(event, overlay)}
        onclick={() => onOverlaySelect(overlay.id)}
      >
        {#if overlay.type === "sting"}
          <StingPreview
            src={projectAssetPreviewUrl(
              projectPath,
              overlay.asset.preview.relativePath,
            )}
            preview={overlay.asset.preview}
            startMs={overlay.startMs}
            {playheadMs}
            playbackRate={stingPlaybackRate(overlay)}
            repeat={stingRepeats(overlay)}
            onReady={() => onPreviewError(null)}
            onError={() =>
              onPreviewError("The generated Skull'd sting preview could not be displayed.")}
          />
        {:else}
          <img
            src={projectAssetPreviewUrl(
              projectPath,
              overlayAsset(overlay).relativePath,
            )}
            alt=""
            draggable="false"
            onerror={() =>
              onPreviewError("A project overlay could not be displayed.")}
          />
        {/if}
        <span
          class="overlay-resize-handle"
          role="presentation"
          onpointerdown={(event) => beginOverlayDrag(event, overlay, "resize")}
        ></span>
      </button>
    {/each}
  {:else}
    <div class="preview-unavailable">
      <strong>Preview unavailable</strong>
      <span>
        {sourceStatus === "missing"
          ? "Relink the missing source video."
          : "Verify or explicitly replace the changed source video."}
      </span>
    </div>
  {/if}
</div>
