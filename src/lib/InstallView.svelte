<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { open } from '@tauri-apps/plugin-dialog';

  let { onInstallStart }: { onInstallStart: (dir: string) => void } = $props();

  let installDir = $state('/home/navitank/Games/Evolve');
  let installing = $state(false);

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

  <div class="disk-hint">~41 GB required</div>

  <button class="install-btn" onclick={install} disabled={installing || !installDir.trim()}>
    {installing ? 'STARTING...' : 'INSTALL'}
  </button>
</div>
