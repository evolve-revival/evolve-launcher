<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import type { DonorStatus } from '../types';

  let { onDone }: { onDone: () => void } = $props();

  type Phase = 'checking' | 'need-donor' | 'copying' | 'ready' | 'no-steam' | 'error';

  let phase = $state<Phase>('checking');
  let donorName = $state('');
  let donorAppId = $state(0);
  let errorMsg = $state('');

  async function check() {
    phase = 'checking';
    try {
      const status = await invoke<DonorStatus>('check_donor_game');
      donorName = status.donor_name;
      donorAppId = status.donor_app_id;
      if (!status.installed) {
        phase = 'need-donor';
      } else if (!status.dll_ready) {
        phase = 'copying';
        await invoke('launch_game').catch(() => {});
        phase = 'ready';
      } else {
        phase = 'ready';
      }
    } catch (e) {
      const msg = String(e);
      if (msg.includes('Steam not found')) {
        phase = 'no-steam';
      } else {
        errorMsg = msg;
        phase = 'error';
      }
    }
  }

  function openDonorStore() {
    invoke('open_steam_store', { appId: donorAppId }).catch(() => {});
    const interval = setInterval(async () => {
      const status = await invoke<DonorStatus>('check_donor_game');
      if (status.installed) {
        clearInterval(interval);
        check();
      }
    }, 3000);
  }

  onMount(check);
</script>

<div class="steam-setup">
  <div class="title">Steam Setup</div>

  {#if phase === 'checking' || phase === 'copying'}
    <div class="body">
      <div class="spinner"></div>
      <span class="hint">{phase === 'copying' ? 'Preparing Steam files…' : 'Checking Steam…'}</span>
    </div>

  {:else if phase === 'need-donor'}
    <div class="body">
      <p class="subtitle">
        To enable Steam multiplayer (SDR, overlay, invites), you need one free Steam game installed:
        <strong>{donorName}</strong>.
      </p>
      <button class="primary-btn" onclick={openDonorStore}>
        Add to Steam Library (Free)
      </button>
      <span class="hint">Click above — it's free. Once installed, this step completes automatically.</span>
    </div>

  {:else if phase === 'ready'}
    <div class="body">
      <div class="check-icon">✓</div>
      <p class="subtitle">Steam is ready. You'll get overlay, invites, and relay networking.</p>
    </div>

  {:else if phase === 'no-steam'}
    <div class="body">
      <p class="subtitle">
        Steam was not found. Install Steam and log in to enable multiplayer features.
      </p>
    </div>

  {:else if phase === 'error'}
    <div class="body">
      <p class="error-msg">{errorMsg}</p>
      <button class="primary-btn" onclick={check}>Retry</button>
    </div>
  {/if}

  <button class="skip-btn" onclick={onDone}>
    {phase === 'ready' ? 'Continue' : 'Skip'}
  </button>
</div>

<style>
  .steam-setup {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100vh;
    padding: 32px;
    gap: 24px;
    color: #fff;
    background: #0f0f12;
  }

  .title {
    font-size: 22px;
    font-weight: 700;
    letter-spacing: 0.12em;
    text-transform: uppercase;
  }

  .body {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 16px;
    width: 100%;
    max-width: 420px;
  }

  .subtitle {
    text-align: center;
    color: #aaa;
    font-size: 14px;
    line-height: 1.6;
    margin: 0;
  }

  .primary-btn {
    background: #4ade80;
    border: none;
    color: #000;
    padding: 10px 28px;
    border-radius: 6px;
    font-size: 14px;
    font-weight: 600;
    cursor: pointer;
    transition: opacity 0.15s;
  }

  .primary-btn:hover { opacity: 0.85; }

  .check-icon {
    font-size: 40px;
    color: #4ade80;
  }

  .hint {
    font-size: 12px;
    color: #666;
    text-align: center;
  }

  .error-msg {
    color: #f87171;
    font-size: 13px;
    text-align: center;
  }

  .skip-btn {
    background: transparent;
    border: 1px solid #333;
    color: #888;
    padding: 8px 28px;
    border-radius: 6px;
    font-size: 13px;
    cursor: pointer;
    transition: border-color 0.15s, color 0.15s;
  }

  .skip-btn:hover { border-color: #888; color: #ccc; }

  .spinner {
    width: 28px;
    height: 28px;
    border: 3px solid #2e2e38;
    border-top-color: #4ade80;
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }

  @keyframes spin { to { transform: rotate(360deg); } }
</style>
