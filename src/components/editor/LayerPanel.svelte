<script lang="ts">
  import type { Overlay } from "../../../contracts/types";
  import { formatTimelineTime } from "../../services/timeline";

  interface Props {
    overlays: Overlay[];
    sourceFilename: string;
    selectedOverlayId: string | null;
    busy?: boolean;
    onSelect: (id: string | null) => void;
    onAddImage: () => Promise<void>;
    onAddCaption: (text: string) => Promise<boolean>;
  }

  let {
    overlays,
    sourceFilename,
    selectedOverlayId,
    busy = false,
    onSelect,
    onAddImage,
    onAddCaption,
  }: Props = $props();

  let composingCaption = $state(false);
  let captionText = $state("");
  let addingCaption = $state(false);

  const orderedOverlays = $derived(
    [...overlays].sort((a, b) => b.zIndex - a.zIndex),
  );

  async function createCaption(): Promise<void> {
    if (!captionText.trim() || addingCaption) {
      return;
    }
    addingCaption = true;
    const added = await onAddCaption(captionText);
    addingCaption = false;
    if (added) {
      captionText = "";
      composingCaption = false;
    }
  }
</script>

<div class="layer-heading">
  <div>
    <p class="section-label">Project</p>
    <h2>Layers</h2>
  </div>
  <span>{overlays.length}/100</span>
</div>

<div class="layer-source">
  <span class="layer-type">source</span>
  <strong>Source video</strong>
  <small>{sourceFilename}</small>
</div>

<div class="layer-actions">
  <button type="button" disabled={busy} onclick={onAddImage}>Add image</button>
  <button
    type="button"
    disabled={busy}
    aria-expanded={composingCaption}
    onclick={() => (composingCaption = !composingCaption)}
  >
    Add caption
  </button>
</div>

{#if composingCaption}
  <div class="caption-composer">
    <label>
      <span>Caption text</span>
      <textarea
        maxlength="500"
        rows="4"
        placeholder="Type the words that should appear on the clip"
        bind:value={captionText}
      ></textarea>
    </label>
    <div>
      <button
        type="button"
        disabled={!captionText.trim() || addingCaption}
        onclick={createCaption}
      >
        {addingCaption ? "Rendering…" : "Add to clip"}
      </button>
      <button
        type="button"
        disabled={addingCaption}
        onclick={() => {
          composingCaption = false;
          captionText = "";
        }}
      >
        Cancel
      </button>
    </div>
  </div>
{/if}

{#if overlays.length === 0}
  <div class="empty-panel">
    <strong>No overlays</strong>
    <span>Add a caption or static brand image when the framing is ready.</span>
  </div>
{:else}
  <div class="layer-list" aria-label="Project overlays">
    {#each orderedOverlays as overlay (overlay.id)}
      <button
        type="button"
        class:selected={selectedOverlayId === overlay.id}
        aria-pressed={selectedOverlayId === overlay.id}
        onclick={() => onSelect(overlay.id)}
      >
        <span class="layer-type">{overlay.type}</span>
        <strong>{overlay.name}</strong>
        <small>
          {formatTimelineTime(overlay.startMs)}–{formatTimelineTime(overlay.endMs)}
        </small>
      </button>
    {/each}
  </div>
{/if}
