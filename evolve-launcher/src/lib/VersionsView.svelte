<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import type { VersionInfo, InstallStatus } from '../types';

  let { onBack, onSwitched }: {
    onBack: () => void;
    onSwitched: (status: InstallStatus) => void;
  } = $props();

  let versions = $state<VersionInfo[]>([]);
  let loading = $state(true);
  let switching = $state('');

  onMount(async () => {
    versions = await invoke<VersionInfo[]>('get_versions');
    loading = false;
  });

  async function switchTo(id: string) {
    switching = id;
    try {
      const status = await invoke<InstallStatus>('switch_version', { id });
      onSwitched(status);
    } catch (e) {
      console.error(e);
      switching = '';
    }
  }

  function stateLabel(v: VersionInfo): string {
    if (v.is_active) return 'Active';
    if (v.state === 'ready') return 'Installed';
    if (v.state === 'paused') return 'Paused';
    return 'Not Installed';
  }
</script>

<div class="versions-view">
  <div class="header">
    <button class="back-btn" onclick={onBack}>← Back</button>
    <span class="page-title">VERSIONS</span>
    <span></span>
  </div>

  {#if loading}
    <div class="loading"><div class="spinner"></div></div>
  {:else}
    <div class="card-list">
      {#each versions as v}
        <div class="card" class:is-active={v.is_active}>
          <div class="card-info">
            <div class="card-row">
              <span class="version-name">{v.name}</span>
              <span
                class="state-badge"
                class:badge-active={v.is_active}
                class:badge-ready={v.state === 'ready' && !v.is_active}
                class:badge-paused={v.state === 'paused'}
              >{stateLabel(v)}</span>
            </div>
            {#if v.install_dir}
              <span class="install-path">{v.install_dir}</span>
            {:else}
              <span class="install-path muted">Not installed</span>
            {/if}
            {#if v.installed_build != null}
              <span class="build-hint">Build {v.installed_build}</span>
            {/if}
          </div>
          <div class="card-actions">
            {#if !v.is_active}
              {#if v.state === 'not-installed'}
                <button class="action-btn install" onclick={() => switchTo(v.id)} disabled={switching === v.id}>
                  {switching === v.id ? '…' : 'Install'}
                </button>
              {:else}
                <button class="action-btn switch" onclick={() => switchTo(v.id)} disabled={switching === v.id}>
                  {switching === v.id ? '…' : 'Switch'}
                </button>
              {/if}
            {/if}
          </div>
        </div>
      {/each}
    </div>
  {/if}

  <div class="footer-note">
    Each version has its own install folder under the Evolve Revival umbrella.
  </div>
</div>

<style>
  .versions-view {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: #0f0f12;
    color: #fff;
    padding: 0;
  }

  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 20px 24px 16px;
    border-bottom: 1px solid #1e1e26;
  }

  .back-btn {
    background: transparent;
    border: 1px solid #2e2e38;
    color: #888;
    padding: 6px 14px;
    border-radius: 5px;
    font-size: 13px;
    cursor: pointer;
    transition: border-color 0.15s, color 0.15s;
  }
  .back-btn:hover { border-color: #888; color: #ccc; }

  .page-title {
    font-size: 16px;
    font-weight: 700;
    letter-spacing: 0.14em;
  }

  .loading {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .spinner {
    width: 28px;
    height: 28px;
    border: 3px solid #2e2e38;
    border-top-color: #4ade80;
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }
  @keyframes spin { to { transform: rotate(360deg); } }

  .card-list {
    flex: 1;
    overflow-y: auto;
    padding: 20px 24px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .card {
    background: #16161d;
    border: 1px solid #2e2e38;
    border-radius: 8px;
    padding: 16px 18px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    transition: border-color 0.15s;
  }
  .card.is-active {
    border-color: #4ade8044;
    background: #16201a;
  }

  .card-info {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
  }

  .card-row {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .version-name {
    font-size: 15px;
    font-weight: 600;
  }

  .state-badge {
    font-size: 11px;
    font-weight: 600;
    padding: 2px 8px;
    border-radius: 4px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    background: #2e2e38;
    color: #888;
  }
  .state-badge.badge-active { background: #1a3d27; color: #4ade80; }
  .state-badge.badge-ready  { background: #1e2d1e; color: #86efac; }
  .state-badge.badge-paused { background: #2e2a1a; color: #fbbf24; }

  .install-path {
    font-size: 12px;
    color: #666;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 340px;
  }
  .install-path.muted { color: #444; }

  .build-hint {
    font-size: 11px;
    color: #444;
  }

  .card-actions {
    flex-shrink: 0;
  }

  .action-btn {
    padding: 7px 18px;
    border-radius: 5px;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    border: none;
    transition: opacity 0.15s;
  }
  .action-btn:disabled { opacity: 0.5; cursor: default; }
  .action-btn.install { background: #4ade80; color: #000; }
  .action-btn.install:hover:not(:disabled) { opacity: 0.85; }
  .action-btn.switch { background: #3b82f6; color: #fff; }
  .action-btn.switch:hover:not(:disabled) { opacity: 0.85; }

  .footer-note {
    padding: 14px 24px;
    font-size: 11px;
    color: #444;
    border-top: 1px solid #1e1e26;
    text-align: center;
  }
</style>
