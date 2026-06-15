<script lang="ts">
  let { items }: { items: { label: string; href?: string }[] } = $props();
</script>

<nav class="flex items-center gap-1.5 text-sm">
  {#each items as item, i}
    {#if i > 0}
      <span class="text-border-default">›</span>
    {/if}
    {#if item.href}
      <a href={item.href} class="text-text-muted hover:text-text-secondary transition-colors">
        {item.label}
      </a>
    {:else}
      <span class="text-text-secondary font-semibold">{item.label}</span>
    {/if}
  {/each}
</nav>
