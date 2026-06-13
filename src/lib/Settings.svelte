<script lang="ts">
  let { onBack }: { onBack: () => void } = $props();
</script>
<div class="settings"><button onclick={onBack}>Back</button></div>
