<script lang="ts">
  let { onInstallStart }: { onInstallStart: (dir: string) => void } = $props();
</script>
<div class="install-view"><button onclick={() => onInstallStart('/tmp/test')}>stub</button></div>
