<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import type { Config } from '../types';

  let { onSettings }: { onSettings: () => void } = $props();

  type Status = 'checking' | 'online' | 'degraded' | 'offline';

  // Stubs — replaced by real polling in later tasks
  let status = $state<Status>('online');
  let playerCount = $state(0);
  let filesVerified = $state(true);

  let gameExeSet = $state(false);
  let launching = $state(false);
  let launchError = $state('');

  const canPlay = $derived(
    (status === 'online' || status === 'degraded') && filesVerified && gameExeSet
  );

  const dotColor = $derived(
    status === 'online'    ? '#4ade80' :
    status === 'degraded'  ? '#fbbf24' : '#ef4444'
  );

  onMount(async () => {
    const cfg = await invoke<Config>('get_config');
    gameExeSet = cfg.game_exe !== '';
  });

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
</script>

<div class="launcher">
  <span class="version-badge">v0.1.0</span>

  <div class="title-block">
    <span class="title-main">EVOLVE</span>
    <span class="title-sub">REVIVAL</span>
  </div>

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

  {#if !gameExeSet}
    <div class="hint">Set game path in Settings to enable play</div>
  {/if}

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
    <button class="settings-btn" onclick={onSettings}>Settings</button>
  </div>
</div>
