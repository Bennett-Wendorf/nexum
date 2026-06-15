<script lang="ts">
  import MarkdownIt from 'markdown-it';
  
  const md = new MarkdownIt({
    html: false,
    breaks: true,
    linkify: true,
    typographer: true,
  });
  
  let { content, className = '' }: { content: string; className?: string } = $props();
  
  const html = $derived(content.trim() ? md.render(content) : '');
  const containerClass = $derived(className || 'md-content');
</script>

<div class={containerClass}>
  {#if html}
    {@html html}
  {:else}
    <p class="text-text-muted italic">No content</p>
  {/if}
</div>
