<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import type { Tier } from '../types';

  let { onBack, onAdvanced, onSaved }: {
    onBack: () => void;
    onAdvanced: () => void;
    onSaved: (tier: Tier) => void;
  } = $props();

  let tiers = $state<Tier[]>([]);
  let loading = $state(true);
  let error = $state('');
  let saving = $state(false);
  let selectedId = $state('');

  onMount(async () => {
    try {
      tiers = await invoke<Tier[]>('get_tiers');
      const def = tiers.find(t => t.selected) ?? tiers.find(t => t.recommended) ?? tiers[0];
      if (def) selectedId = def.id;
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  });

  const selectedTier = $derived(tiers.find(t => t.id === selectedId));

  function fmtGb(bytes: number): string {
    return (bytes / 1_073_741_824).toFixed(1) + ' GB';
  }

  async function confirm() {
    if (!selectedTier) return;
    saving = true;
    try {
      await invoke('save_tier', { tierId: selectedTier.id, components: selectedTier.components });
      onSaved(selectedTier);
    } catch (e) {
      error = String(e);
      saving = false;
    }
  }
</script>

<div class="tier-view">
  <span class="version-badge">v0.1.0</span>

  <div class="tier-header">
    <span class="tier-title">Install Type</span>
  </div>

  {#if loading}
    <div class="tier-loading">Loading…</div>
  {:else if error}
    <div class="tier-error">{error}</div>
    <div class="tier-footer">
      <button class="tier-cancel" onclick={onBack}>Back</button>
    </div>
  {:else if tiers.length === 0}
    <div class="tier-empty">No install tiers defined — using manifest defaults.</div>
    <div class="tier-footer">
      <button class="tier-cancel" onclick={onBack}>Back</button>
    </div>
  {:else}
    <div class="tier-list">
      {#each tiers as tier (tier.id)}
        <label class="tier-option" class:active={selectedId === tier.id}>
          <input
            type="radio"
            name="tier"
            value={tier.id}
            bind:group={selectedId}
          />
          <div class="tier-info">
            <div class="tier-name-row">
              <span class="tier-name">{tier.name}</span>
              {#if tier.recommended}
                <span class="tier-badge">Recommended</span>
              {/if}
              <span class="tier-size">{fmtGb(tier.size_bytes)}</span>
            </div>
            <span class="tier-desc">{tier.description}</span>
          </div>
        </label>
      {/each}
    </div>

    <button class="tier-advanced" onclick={onAdvanced}>Advanced ›</button>

    <div class="tier-footer">
      <button class="tier-cancel" onclick={onBack}>Back</button>
      <button class="tier-confirm" onclick={confirm} disabled={saving || !selectedTier}>
        {saving ? 'Saving…' : 'Confirm'}
      </button>
    </div>
  {/if}
</div>

<style>
  .tier-view {
    display: flex;
    flex-direction: column;
    height: 100vh;
    padding: 14px 20px 16px;
    gap: 10px;
    background: #0f0f1a;
    color: #e0e0e0;
    font-family: 'Segoe UI', system-ui, sans-serif;
    position: relative;
  }

  .version-badge {
    position: absolute;
    top: 10px;
    left: 12px;
    font-size: 10px;
    color: rgba(255, 255, 255, 0.2);
    pointer-events: none;
  }

  .tier-header {
    padding-top: 10px;
    display: flex;
    align-items: center;
  }

  .tier-title {
    font-size: 14px;
    font-weight: 600;
    color: #ccc;
    letter-spacing: 0.05em;
  }

  .tier-loading, .tier-empty {
    font-size: 12px;
    color: #666;
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .tier-error {
    font-size: 11px;
    color: #ef4444;
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    text-align: center;
  }

  .tier-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
    flex: 1;
  }

  .tier-option {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 9px 10px;
    border: 1px solid #1e1e30;
    border-radius: 4px;
    cursor: pointer;
    transition: border-color 0.12s, background 0.12s;
  }
  .tier-option:hover { border-color: #333; }
  .tier-option.active {
    border-color: #e94560;
    background: rgba(233, 69, 96, 0.06);
  }

  .tier-option input[type="radio"] {
    margin-top: 3px;
    accent-color: #e94560;
    flex-shrink: 0;
  }

  .tier-info {
    display: flex;
    flex-direction: column;
    gap: 3px;
    flex: 1;
    min-width: 0;
  }

  .tier-name-row {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .tier-name {
    font-size: 12px;
    font-weight: 600;
    color: #ddd;
  }

  .tier-badge {
    font-size: 9px;
    font-weight: 700;
    color: #f97316;
    background: rgba(249, 115, 22, 0.12);
    border: 1px solid rgba(249, 115, 22, 0.3);
    border-radius: 3px;
    padding: 1px 5px;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .tier-size {
    font-size: 11px;
    color: #555;
    margin-left: auto;
    white-space: nowrap;
    flex-shrink: 0;
  }

  .tier-desc {
    font-size: 10px;
    color: #666;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .tier-advanced {
    background: none;
    border: none;
    color: #e94560;
    font-size: 11px;
    cursor: pointer;
    padding: 0;
    text-align: center;
    align-self: center;
    text-decoration: underline;
    text-underline-offset: 2px;
    transition: color 0.15s;
  }
  .tier-advanced:hover { color: #ff6b84; }

  .tier-footer {
    display: flex;
    gap: 8px;
  }

  .tier-cancel {
    background: #1e1e30;
    border: 1px solid #2e2e44;
    color: #aaa;
    padding: 8px 14px;
    border-radius: 3px;
    cursor: pointer;
    font-size: 12px;
  }
  .tier-cancel:hover { border-color: #555; color: #ddd; }

  .tier-confirm {
    flex: 1;
    background: #e94560;
    border: 1px solid #e94560;
    color: #fff;
    padding: 8px 14px;
    border-radius: 3px;
    cursor: pointer;
    font-size: 12px;
    font-weight: 600;
  }
  .tier-confirm:disabled { opacity: 0.4; cursor: not-allowed; }
  .tier-confirm:not(:disabled):hover { opacity: 0.85; }
</style>
