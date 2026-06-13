<script lang="ts">
  let { onSettings }: { onSettings: () => void } = $props();
</script>
<div class="launcher"><button onclick={onSettings}>Settings</button></div>
