<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import type { AppState, DownloadProgress } from '../types';

  let { appState, onPause, onResume }: {
    appState: AppState;
    onPause: () => void;
    onResume: () => void;
  } = $props();

  let progress = $state<DownloadProgress>({
    downloaded_bytes: 0,
    total_bytes: 1,
    current_file: '',
    speed_bps: 0,
    eta_secs: 0,
  });

  let unlisten: (() => void) | null = null;

  onMount(async () => {
    unlisten = await listen<DownloadProgress>('download-progress', (event) => {
      progress = event.payload;
    });
  });

  onDestroy(() => { unlisten?.(); });

  const pct = $derived(
    progress.total_bytes > 0
      ? Math.min(100, (progress.downloaded_bytes / progress.total_bytes) * 100)
      : 0
  );

  const gbDone = $derived((progress.downloaded_bytes / 1e9).toFixed(1));
  const gbTotal = $derived((progress.total_bytes / 1e9).toFixed(1));

  const speed = $derived(
    progress.speed_bps >= 1e6
      ? `${(progress.speed_bps / 1e6).toFixed(1)} MB/s`
      : `${(progress.speed_bps / 1e3).toFixed(0)} KB/s`
  );

  const eta = $derived(
    progress.eta_secs >= 3600
      ? `${Math.floor(progress.eta_secs / 3600)}h ${Math.floor((progress.eta_secs % 3600) / 60)}m`
      : progress.eta_secs >= 60
      ? `${Math.floor(progress.eta_secs / 60)}m ${progress.eta_secs % 60}s`
      : `${progress.eta_secs}s`
  );

  const isPaused = $derived(appState === 'paused');
  const label = $derived(
    appState === 'repairing' ? 'REPAIRING' :
    appState === 'paused'    ? 'PAUSED' : 'DOWNLOADING'
  );

  async function togglePause() {
    if (isPaused) {
      await invoke('resume_install');
      onResume();
    } else {
      invoke('pause_install');
      onPause();
    }
  }
</script>

<div class="progress-view">
  <span class="version-badge">v0.1.0</span>

  <div class="install-title-block">
    <span class="title-main">EVOLVE</span>
    <span class="title-sub">REVIVAL</span>
  </div>

  <div class="progress-label">{label}</div>

  <div class="progress-bar-wrap">
    <div class="progress-bar-fill" style="width: {pct}%"></div>
  </div>

  <div class="progress-stats">
    <span>{gbDone} / {gbTotal} GB</span>
    {#if !isPaused}
      <span>{speed}</span>
      <span>ETA {eta}</span>
    {/if}
  </div>

  {#if progress.current_file}
    <div class="progress-file">{progress.current_file}</div>
  {/if}

  <button class="pause-btn" onclick={togglePause}>
    {isPaused ? 'RESUME' : 'PAUSE'}
  </button>
</div>
