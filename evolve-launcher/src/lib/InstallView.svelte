<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { open } from '@tauri-apps/plugin-dialog';

  let { onInstallStart, onCustomize, selectedBytes }: {
    onInstallStart: (dir: string) => void;
    onCustomize: () => void;
    selectedBytes: number | null;
  } = $props();

  let installDir = $state('');
  let installing = $state(false);

  function fmtGb(bytes: number): string {
    return (bytes / 1_073_741_824).toFixed(1) + ' GB';
  }

  const diskHint = $derived(
    selectedBytes !== null ? `~${fmtGb(selectedBytes)} required` : '~41 GB required'
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
    <button class="customize-btn" onclick={onCustomize}>Customize</button>
  </div>

  <button class="install-btn" onclick={install} disabled={installing || !installDir.trim()}>
    {installing ? 'STARTING...' : 'INSTALL'}
  </button>
</div>
