<script lang="ts">
  import type { StingPreviewRef } from "../../../contracts/types";

  interface Props {
    src: string;
    preview: StingPreviewRef;
    startMs: number;
    playheadMs: number;
    onReady: () => void;
    onError: () => void;
  }

  let {
    src,
    preview,
    startMs,
    playheadMs,
    onReady,
    onError,
  }: Props = $props();

  const frameIndex = $derived(
    Math.min(
      preview.frameCount - 1,
      Math.max(
        0,
        Math.floor(
          ((playheadMs - startMs) / 1_000) * preview.framesPerSecond,
        ),
      ),
    ),
  );
  const column = $derived(frameIndex % preview.columns);
  const row = $derived(Math.floor(frameIndex / preview.columns));
  const backgroundX = $derived(
    preview.columns === 1 ? 0 : (column / (preview.columns - 1)) * 100,
  );
  const backgroundY = $derived(
    preview.rows === 1 ? 0 : (row / (preview.rows - 1)) * 100,
  );
  const spriteStyle = $derived(
    [
      `background-image:url("${src}")`,
      `background-size:${preview.columns * 100}% ${preview.rows * 100}%`,
      `background-position:${backgroundX}% ${backgroundY}%`,
    ].join(";"),
  );
</script>

<img
  class="sting-preview-loader"
  {src}
  alt=""
  draggable="false"
  onload={onReady}
  onerror={onError}
/>
<span class="sting-preview-sprite" style={spriteStyle} aria-hidden="true"></span>
