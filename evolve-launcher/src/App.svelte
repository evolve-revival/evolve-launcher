<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import type { AppState, InstallStatus, Tier } from './types';
  import Main from './lib/Main.svelte';
  import Settings from './lib/Settings.svelte';
  import InstallView from './lib/InstallView.svelte';
  import TierView from './lib/TierView.svelte';
  import ProgressView from './lib/ProgressView.svelte';
  import ComponentsView from './lib/ComponentsView.svelte';
  import SteamSetupView from './lib/SteamSetupView.svelte';
  import VersionsView from './lib/VersionsView.svelte';

  type View = 'main' | 'settings' | 'tiers' | 'components' | 'versions';

  let appState = $state<AppState>('not-installed');
  let view = $state<View>('main');
  let installDir = $state('');
  let selectedTier = $state<Tier | null>(null);
  let selectedBytes = $state<number | null>(null);

  onMount(async () => {
    const status = await invoke<InstallStatus>('check_install_state');
    appState = status.state;
    installDir = status.install_dir;

    await listen('install-complete', () => { appState = 'steam-setup'; });
    await listen('repair-complete',  () => { appState = 'ready'; });
    await listen('install-paused',   () => { appState = 'paused'; });
    await listen('install-error',    () => { appState = installDir ? 'paused' : 'not-installed'; });
    await listen('game-launched',    () => { appState = 'playing'; });
    await listen('game-exited',      () => { appState = 'ready'; });

    if (appState === 'ready') {
      invoke<boolean>('check_for_updates').then(hasUpdate => {
        if (hasUpdate) appState = 'update-available';
      }).catch(() => {});
    }
  });

  function onInstallStart(dir: string) {
    installDir = dir;
    appState = 'downloading';
  }

  function onPause() {
    appState = 'paused';
  }

  function onResume() {
    appState = 'downloading';
  }

  function onRepair() {
    appState = 'repairing';
  }

  function onTierSaved(tier: Tier) {
    selectedTier = tier;
    selectedBytes = tier.size_bytes;
    view = 'main';
  }

  function onComponentsSaved(totalBytes: number) {
    selectedTier = null;
    selectedBytes = totalBytes;
    view = 'main';
  }

  function onVersionSwitched(status: InstallStatus) {
    appState = status.state;
    installDir = status.install_dir;
    view = 'main';
  }
</script>

{#if appState === 'steam-setup'}
  <SteamSetupView onDone={() => { appState = 'ready'; }} />
{:else if appState === 'not-installed'}
  {#if view === 'tiers'}
    <TierView
      onBack={() => (view = 'main')}
      onAdvanced={() => (view = 'components')}
      onSaved={onTierSaved}
    />
  {:else if view === 'components'}
    <ComponentsView
      onBack={() => (view = 'tiers')}
      onSaved={onComponentsSaved}
    />
  {:else}
    <InstallView
      onInstallStart={onInstallStart}
      onChooseTier={() => (view = 'tiers')}
      {selectedTier}
      {selectedBytes}
    />
  {/if}
{:else if appState === 'downloading' || appState === 'repairing' || appState === 'paused'}
  <ProgressView
    {appState}
    onPause={onPause}
    onResume={onResume}
  />
{:else if view === 'versions'}
  <VersionsView onBack={() => (view = 'main')} onSwitched={onVersionSwitched} />
{:else if view === 'settings'}
  <Settings onBack={() => (view = 'main')} />
{:else}
  <Main
    {appState}
    onSettings={() => (view = 'settings')}
    onRepair={onRepair}
    onVersions={() => (view = 'versions')}
  />
{/if}
