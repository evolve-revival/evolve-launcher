<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { open } from '@tauri-apps/plugin-dialog';
  import type { Tier } from '../types';

  let { onInstallStart, onChooseTier, selectedTier, selectedBytes }: {
    onInstallStart: (dir: string) => void;
    onChooseTier: () => void;
    selectedTier: Tier | null;
    selectedBytes: number | null;
  } = $props();

  let installDir = $state('');
  let installing = $state(false);

  function fmtGb(bytes: number): string {
    return (bytes / 1_073_741_824).toFixed(1) + ' GB';
  }

  const diskHint = $derived(
    selectedTier
      ? `${selectedTier.name}  —  ~${fmtGb(selectedTier.size_bytes)}`
      : selectedBytes !== null
      ? `Custom  —  ~${fmtGb(selectedBytes)}`
      : '~41 GB required'
  );

  const chooseBtnLabel = $derived(
    selectedTier !== null || selectedBytes !== null ? 'Change' : 'Choose'
  );

  async function browseDir() {
    const result = await open({ directory: true, title: 'Select Install Folder' });
    if (typeof result === 'string') installDir = result;
  }

  async function install() {
    if (!installDir.trim()) return;
    installing = true;
    try {
      await invoke('start_install', { installDir: installDir.trim() });
      onInstallStart(installDir.trim());
    } catch (e) {
      console.error('install failed:', e);
      installing = false;
    }
  }
</script>

<div class="install-view">
  <span class="version-badge">v0.1.0</span>

  <div class="install-title-block">
    <span class="title-main">EVOLVE</span>
    <span class="title-sub">REVIVAL</span>
  </div>

  <div class="install-label">Install Location</div>

  <div class="dir-row">
    <input
      class="dir-input"
      type="text"
      bind:value={installDir}
      placeholder="/home/user/Games/Evolve"
    />
    <button class="dir-browse-btn" onclick={browseDir}>Browse</button>
  </div>

  <div class="disk-row">
    <span class="disk-hint">{diskHint}</span>
    <button class="customize-btn" onclick={onChooseTier}>{chooseBtnLabel}</button>
  </div>

  <button class="install-btn" onclick={install} disabled={installing || !installDir.trim()}>
    {installing ? 'STARTING...' : 'INSTALL'}
  </button>
</div>
