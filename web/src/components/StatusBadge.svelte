<script lang="ts">
  import { getPlanStatusColor, getTaskStatusColor, kanbanColumnLabels } from '$lib/statusColors';
  
  let { status, type, label = undefined }: {
    status: string;
    type: 'plan' | 'task';
    label?: string;
  } = $props();
  
  const badgeClass = $derived(type === 'plan' ? getPlanStatusColor(status) : getTaskStatusColor(status));
  const displayLabel = $derived(label ?? (kanbanColumnLabels[status] ?? status));
</script>

<span class="inline-flex items-center px-2 py-0.5 rounded-full text-xs font-semibold {badgeClass}">
  {displayLabel}
</span>
