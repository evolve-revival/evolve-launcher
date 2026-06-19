<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';
  import type { AppState } from '../types';

  let { appState, onSettings, onRepair, onVersions }: {
    appState: AppState;
    onSettings: () => void;
    onRepair: () => void;
    onVersions: () => void;
  } = $props();

  type Status = 'checking' | 'online' | 'degraded' | 'offline';

  let status = $state<Status>('online');
  let playerCount = $state(0);
  let filesVerified = $state(true);
  let launching = $state(false);
  let launchError = $state('');


  const canPlay = $derived(
    (status === 'online' || status === 'degraded') && filesVerified
  );

  const dotColor = $derived(
    status === 'online'   ? '#4ade80' :
    status === 'degraded' ? '#fbbf24' : '#ef4444'
  );

  async function play() {
    launchError = '';
    launching = true;
    try {
      await invoke('launch_game');
    } catch (e) {
      launchError = String(e);
    } finally {
      launching = false;
    }
  }

  async function update() {
    await invoke('start_update');
    onRepair();
  }
</script>

<div class="launcher">
  <span class="version-badge">v0.1.0</span>

  <div class="title-block">
    <span class="title-main">EVOLVE</span>
    <span class="title-sub">REVIVAL</span>
  </div>

  {#if appState === 'update-available'}
    <div class="update-banner">
      Update available
      <button class="update-btn" onclick={update}>UPDATE</button>
    </div>
  {/if}

  <div class="status-row">
    <span class="dot" style="background: {dotColor}; color: {dotColor}"></span>
    {#if status === 'online'}
      ONLINE &nbsp;·&nbsp; {playerCount} players
    {:else if status === 'degraded'}
      DEGRADED &nbsp;·&nbsp; {playerCount} players
    {:else if status === 'checking'}
      CHECKING...
    {:else}
      OFFLINE
    {/if}
  </div>

  {#if launchError}
    <div class="launch-error">{launchError}</div>
  {/if}

  <button class="play-btn" onclick={play} disabled={!canPlay || launching || appState === 'playing'} class:playing={appState === 'playing'}>
    {appState === 'playing' ? 'PLAYING' : launching ? 'LAUNCHING...' : 'PLAY'}
  </button>

  <div class="footer">
    <span class="verify-status">
      {#if filesVerified}
        <span class="check">✓</span>Files verified
      {:else}
        <span class="cross">✗</span>Files not verified
      {/if}
    </span>
    <div style="display:flex; gap:14px; align-items:center;">
      <button class="repair-btn" onclick={async () => { await invoke('start_repair'); onRepair(); }}>Repair</button>
      <button class="versions-btn" onclick={onVersions}>Versions</button>
      <button class="settings-btn" onclick={onSettings}>Settings</button>
    </div>
  </div>
</div>

<style>
  .versions-btn {
    background: transparent;
    border: 1px solid #2e2e38;
    color: #888;
    padding: 5px 12px;
    border-radius: 5px;
    font-size: 12px;
    cursor: pointer;
    transition: border-color 0.15s, color 0.15s;
  }
  .versions-btn:hover { border-color: #888; color: #ccc; }
</style>
