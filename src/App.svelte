<script lang="ts">
  import { onMount } from "svelte";

  import type { AppError, RuntimeInfo } from "./contracts/runtime";
  import { getRuntimeInfo, normalizeAppError } from "./services/tauri";

  let runtime = $state<RuntimeInfo | null>(null);
  let error = $state<AppError | null>(null);
  let loading = $state(true);

  const platformLabel = $derived(
    runtime ? `${runtime.os} · ${runtime.arch}` : "Detecting local runtime",
  );

  async function loadRuntime(): Promise<void> {
    loading = true;
    error = null;

    try {
      runtime = await getRuntimeInfo();
    } catch (reason) {
      runtime = null;
      error = normalizeAppError(reason);
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    void loadRuntime();
  });
</script>

<svelte:head>
  <title>Skull’d Clip Forge</title>
</svelte:head>

<main>
  <header class="masthead">
    <a class="brand" href="/" aria-label="Skull’d Clip Forge home">
      <span class="brand-mark" aria-hidden="true">SCF</span>
      <span>
        <strong>Skull’d</strong>
        <small>Clip Forge</small>
      </span>
    </a>
    <span class="local-badge">Local only</span>
  </header>

  <section class="hero" aria-labelledby="page-title">
    <p class="eyebrow">Gameplay in. Vertical clip out.</p>
    <h1 id="page-title">Forge the moment.<br />Skip the editing suite.</h1>
    <p class="intro">
      A focused desktop workflow for turning one local gameplay clip into a
      branded 9:16 MP4. No account, upload, cloud, or publishing API.
    </p>
  </section>

  <section class="runtime-card" aria-labelledby="runtime-heading">
    <div class="runtime-heading">
      <div>
        <p class="section-label">Native boundary</p>
        <h2 id="runtime-heading">Runtime readiness</h2>
      </div>
      <span class:ready={runtime !== null} class:error={error !== null} class="status-dot">
        {#if loading}
          Checking
        {:else if runtime}
          Ready
        {:else}
          Needs attention
        {/if}
      </span>
    </div>

    {#if loading}
      <div class="loading-state" aria-live="polite">
        <span class="spinner" aria-hidden="true"></span>
        <span>Verifying the Rust and media-tool boundary…</span>
      </div>
    {:else if error}
      <div class="error-state" role="alert">
        <strong>{error.message}</strong>
        {#if error.safeDetail}
          <p>{error.safeDetail}</p>
        {/if}
        <code>{error.code}</code>
        {#if error.retryable}
          <button type="button" onclick={loadRuntime}>Retry check</button>
        {/if}
      </div>
    {:else if runtime}
      <dl class="runtime-grid">
        <div>
          <dt>Application</dt>
          <dd>v{runtime.appVersion}</dd>
        </div>
        <div>
          <dt>Platform</dt>
          <dd>{platformLabel}</dd>
        </div>
        <div>
          <dt>ffmpeg</dt>
          <dd>{runtime.ffmpegVersion}</dd>
        </div>
        <div>
          <dt>ffprobe</dt>
          <dd>{runtime.ffprobeVersion}</dd>
        </div>
        <div>
          <dt>Project schema</dt>
          <dd>v{runtime.projectSchemaVersion}</dd>
        </div>
        <div>
          <dt>Media tools</dt>
          <dd>{runtime.bundledSidecars ? "Pinned sidecars" : "Development paths"}</dd>
        </div>
      </dl>
      <p class="boundary-note">
        Process execution stays in Rust. The webview has no shell permission.
      </p>
    {/if}
  </section>

  <footer>
    <span>Milestone 0 · Scaffold and boundary</span>
    <span>Offline by design</span>
  </footer>
</main>
