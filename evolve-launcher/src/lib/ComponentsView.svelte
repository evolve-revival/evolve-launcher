<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import type { Component } from '../types';

  let { onBack, onSaved }: {
    onBack: () => void;
    onSaved: (totalBytes: number) => void;
  } = $props();

  let components = $state<Component[]>([]);
  let loading = $state(true);
  let error = $state('');
  let saving = $state(false);

  onMount(async () => {
    try {
      components = await invoke<Component[]>('get_components');
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  });

  function toggle(id: string) {
    const idx = components.findIndex(c => c.id === id);
    if (idx === -1 || components[idx].required) return;
    components[idx] = { ...components[idx], enabled: !components[idx].enabled };
  }

  const selectedBytes = $derived(
    components.filter(c => c.enabled).reduce((sum, c) => sum + c.size_bytes, 0)
  );

  function fmtGb(bytes: number): string {
    return (bytes / 1_073_741_824).toFixed(1) + ' GB';
  }

  async function save() {
    saving = true;
    try {
      const selected = components.filter(c => c.enabled).map(c => c.id);
      await invoke('save_components', { selected });
      onSaved(selectedBytes);
    } catch (e) {
      error = String(e);
      saving = false;
    }
  }
</script>

<div class="components-view">
  <div class="comp-header">
    <span class="comp-title">Select Components</span>
  </div>

  {#if loading}
    <div class="comp-loading">Loading manifest…</div>
  {:else if error}
    <div class="comp-error">{error}</div>
    <div class="comp-footer">
      <button class="comp-cancel" onclick={onBack}>Back</button>
    </div>
  {:else if components.length === 0}
    <div class="comp-empty">No optional components available.</div>
    <div class="comp-footer">
      <button class="comp-cancel" onclick={onBack}>Back</button>
    </div>
  {:else}
    <div class="comp-list">
      {#each components as comp (comp.id)}
        <label class="comp-row" class:required={comp.required}>
          <input
            type="checkbox"
            checked={comp.enabled}
            disabled={comp.required}
            onchange={() => toggle(comp.id)}
          />
          <div class="comp-info">
            <span class="comp-name">{comp.name}{comp.required ? ' (required)' : ''}</span>
            <span class="comp-desc">{comp.description}</span>
          </div>
          <span class="comp-size">{fmtGb(comp.size_bytes)}</span>
        </label>
      {/each}
    </div>

    <div class="comp-total">
      Total: <strong>{fmtGb(selectedBytes)}</strong>
    </div>

    <div class="comp-footer">
      <button class="comp-cancel" onclick={onBack}>Cancel</button>
      <button class="comp-save" onclick={save} disabled={saving}>
        {saving ? 'Saving…' : 'Save'}
      </button>
    </div>
  {/if}
</div>

<style>
  .components-view {
    display: flex;
    flex-direction: column;
    height: 100vh;
    padding: 20px;
    gap: 12px;
    background: #0f0f1a;
    color: #e0e0e0;
    font-family: 'Segoe UI', system-ui, sans-serif;
  }

  .comp-header { display: flex; align-items: center; }

  .comp-title {
    font-size: 14px;
    font-weight: 600;
    color: #ccc;
    letter-spacing: 0.05em;
  }

  .comp-loading, .comp-empty {
    font-size: 12px;
    color: #666;
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .comp-error {
    font-size: 11px;
    color: #ef4444;
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    text-align: center;
  }

  .comp-list {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .comp-row {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 8px 10px;
    border: 1px solid #1e1e30;
    border-radius: 4px;
    cursor: pointer;
    transition: border-color 0.12s;
  }
  .comp-row:hover:not(.required) { border-color: #333; }
  .comp-row.required { opacity: 0.6; cursor: default; }

  .comp-row input[type="checkbox"] {
    margin-top: 2px;
    accent-color: #e94560;
    flex-shrink: 0;
  }

  .comp-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1;
    min-width: 0;
  }

  .comp-name {
    font-size: 12px;
    font-weight: 600;
    color: #ddd;
  }

  .comp-desc {
    font-size: 10px;
    color: #666;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .comp-size {
    font-size: 11px;
    color: #555;
    white-space: nowrap;
    flex-shrink: 0;
    margin-top: 2px;
  }

  .comp-total {
    font-size: 12px;
    color: #888;
    text-align: right;
  }
  .comp-total strong { color: #e0e0e0; }

  .comp-footer {
    display: flex;
    gap: 8px;
  }

  .comp-cancel {
    background: #1e1e30;
    border: 1px solid #2e2e44;
    color: #aaa;
    padding: 7px 14px;
    border-radius: 3px;
    cursor: pointer;
    font-size: 12px;
  }
  .comp-cancel:hover { border-color: #555; color: #ddd; }

  .comp-save {
    flex: 1;
    background: #e94560;
    border: 1px solid #e94560;
    color: #fff;
    padding: 7px 14px;
    border-radius: 3px;
    cursor: pointer;
    font-size: 12px;
    font-weight: 600;
  }
  .comp-save:disabled { opacity: 0.4; cursor: not-allowed; }
  .comp-save:not(:disabled):hover { opacity: 0.85; }
</style>
