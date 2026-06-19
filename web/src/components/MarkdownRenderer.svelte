<script lang="ts">
  import { renderMarkdown } from '$lib/markdown';
  
  let { content, className = '' }: { content: string; className?: string } = $props();
  
  const html = $derived(renderMarkdown(content.trim()));
  const containerClass = $derived(className || 'md-content');
</script>

<div class={containerClass}>
  {#if html}
    {@html html}
  {:else}
    <p class="text-text-muted italic">No content</p>
  {/if}
</div>
