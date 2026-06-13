<script lang="ts">
  import Main from './lib/Main.svelte';
  import Settings from './lib/Settings.svelte';

  let view = $state<'main' | 'settings'>('main');
</script>

{#if view === 'main'}
  <Main onSettings={() => (view = 'settings')} />
{:else}
  <Settings onBack={() => (view = 'main')} />
{/if}
