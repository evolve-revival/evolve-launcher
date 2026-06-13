<script lang="ts">
  import type { AppState } from '../types';
  let { appState, onPause, onResume }: { appState: AppState; onPause: () => void; onResume: () => void } = $props();
</script>
<div class="progress-view"><button onclick={onPause}>stub</button></div>
