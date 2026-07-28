<script lang="ts">
  import type { Overlay } from "../../../contracts/types";

  interface Props {
    overlays: Overlay[];
    sourceFilename: string;
    selectedOverlayId: string | null;
    busy?: boolean;
    onSelect: (id: string | null) => void;
    onAddImage: () => Promise<void>;
    onAddSting: () => Promise<void>;
    onAddCaption: (text: string) => Promise<boolean>;
  }

  let {
    overlays,
    sourceFilename,
    selectedOverlayId,
    busy = false,
    onSelect,
    onAddImage,
    onAddSting,
    onAddCaption,
  }: Props = $props();

  let composingCaption = $state(false);
  let captionText = $state("");
  let addingCaption = $state(false);

  const orderedOverlays = $derived(
    [...overlays].sort((a, b) => b.zIndex - a.zIndex),
  );
  const hasSting = $derived(overlays.some((overlay) => overlay.type === "sting"));

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

<div class="rail-heading">
  <span>Layers</span>
  <strong>{overlays.length}</strong>
</div>

<button
  type="button"
  class="rail-item rail-source"
  class:selected={selectedOverlayId === null}
  aria-label={`Edit source crop for ${sourceFilename}`}
  aria-pressed={selectedOverlayId === null}
  title={sourceFilename}
  onclick={() => onSelect(null)}
>
  <span class="rail-glyph">VID</span>
  <strong>Source</strong>
</button>

<div class="layer-actions">
  <button
    type="button"
    class="rail-item"
    disabled={busy}
    aria-label="Add image overlay"
    title="Add image overlay"
    onclick={onAddImage}
  >
    <span class="rail-glyph">IMG</span>
    <strong>Image</strong>
  </button>
  <button
    type="button"
    class="rail-item"
    disabled={busy || hasSting}
    aria-label={hasSting ? "Skull’d sting already added" : "Add Skull’d sting"}
    title={hasSting ? "Skull’d sting already added" : "Add Skull’d sting"}
    onclick={onAddSting}
  >
    <span class="rail-glyph">STG</span>
    <strong>Sting</strong>
  </button>
  <button
    type="button"
    class="rail-item"
    disabled={busy}
    aria-expanded={composingCaption}
    aria-label="Add caption overlay"
    title="Add caption overlay"
    onclick={() => (composingCaption = !composingCaption)}
  >
    <span class="rail-glyph">TXT</span>
    <strong>Caption</strong>
  </button>
</div>

{#if composingCaption}
  <div class="caption-composer" role="dialog" aria-label="Add caption">
    <div class="caption-composer-heading">
      <div>
        <p class="section-label">Caption</p>
        <strong>Add text overlay</strong>
      </div>
      <button
        type="button"
        aria-label="Close caption composer"
        disabled={addingCaption}
        onclick={() => {
          composingCaption = false;
          captionText = "";
        }}
      >
        Close
      </button>
    </div>
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
    </div>
  </div>
{/if}

{#if overlays.length > 0}
  <div class="layer-list" aria-label="Project overlays">
    {#each orderedOverlays as overlay (overlay.id)}
      <button
        type="button"
        class="rail-item"
        class:selected={selectedOverlayId === overlay.id}
        aria-label={`Edit ${overlay.type} overlay ${overlay.name}`}
        aria-pressed={selectedOverlayId === overlay.id}
        title={overlay.name}
        onclick={() => onSelect(overlay.id)}
      >
        <span class="rail-glyph">
          {overlay.type === "sting" ? "STG" : overlay.type === "image" ? "IMG" : "TXT"}
        </span>
        <strong>{overlay.type}</strong>
      </button>
    {/each}
  </div>
{/if}
