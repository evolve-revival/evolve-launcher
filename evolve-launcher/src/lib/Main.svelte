<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';
  import type { AppState, NatInfo } from '../types';

  let { appState, onSettings, onRepair }: {
    appState: AppState;
    onSettings: () => void;
    onRepair: () => void;
  } = $props();

  type Status = 'checking' | 'online' | 'degraded' | 'offline';

  let status = $state<Status>('online');
  let playerCount = $state(0);
  let filesVerified = $state(true);
  let launching = $state(false);
  let launchError = $state('');
  let natInfo = $state<NatInfo | null>(null);

  onMount(() => {
    invoke<NatInfo>('get_nat_type').then(info => { natInfo = info; }).catch(() => {});
  });

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
    {#if natInfo !== null}
      <span class="status-sep">&nbsp;·&nbsp;</span>
      <div class="nat-indicator">
        <span class="nat-dot" class:nat-direct={natInfo.nat_type === 'direct'} class:nat-relay={natInfo.nat_type !== 'direct'}></span>
        <span class="nat-label">{natInfo.nat_type === 'direct' ? 'Direct' : 'Relay'}</span>
      </div>
    {/if}
  </div>

  {#if launchError}
    <div class="launch-error">{launchError}</div>
  {/if}

  <button class="play-btn" onclick={play} disabled={!canPlay || launching}>
    {launching ? 'LAUNCHING...' : 'PLAY'}
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
      <button class="settings-btn" onclick={onSettings}>Settings</button>
    </div>
  </div>
</div>

<style>
  .status-sep {
    color: #555;
  }

  .nat-indicator {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 11px;
    color: #888;
  }

  .nat-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: #888;
  }

  .nat-dot.nat-direct {
    background: #4ade80;
  }

  .nat-dot.nat-relay {
    background: #f59e0b;
  }

  .nat-label {
    color: #aaa;
  }
</style>
