<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { open } from '@tauri-apps/plugin-dialog';
  import type { Config } from '../types';

  let { onBack }: { onBack: () => void } = $props();

  let gameExe = $state('');
  let serverUrl = $state('http://localhost:8080');
  let saving = $state(false);
  let saved = $state(false);

  onMount(async () => {
    const cfg = await invoke<Config>('get_config');
    gameExe = cfg.game_exe;
    serverUrl = cfg.server_url;
  });

  async function pickExe() {
    const path = await open({
      title: 'Select Evolve.exe',
      filters: [{ name: 'Executable', extensions: ['exe'] }],
    });
    if (typeof path === 'string') gameExe = path;
  }

  async function save() {
    saving = true;
    try {
      await invoke('save_config', {
        config: { game_exe: gameExe, server_url: serverUrl },
      });
      saved = true;
      setTimeout(() => {
        saved = false;
        onBack();
      }, 600);
    } catch (e) {
      console.error('save_config failed:', e);
    } finally {
      saving = false;
    }
  }
</script>

<div class="settings">
  <div class="settings-title">Settings</div>

  <div class="field">
    <div class="field-label">Game Executable</div>
    <div class="field-row">
      <input
        type="text"
        value={gameExe}
        oninput={(e) => (gameExe = e.currentTarget.value)}
        placeholder="C:\...\Evolve.exe"
      />
      <button class="browse-btn" onclick={pickExe}>Browse</button>
    </div>
  </div>

  <div class="field">
    <div class="field-label">Server URL</div>
    <input type="text" bind:value={serverUrl} />
  </div>

  <div class="settings-footer">
    <button class="cancel-btn" onclick={onBack}>Cancel</button>
    <button class="save-btn" onclick={save} disabled={saving}>
      {saved ? 'Saved!' : saving ? 'Saving...' : 'Save'}
    </button>
  </div>
</div>
