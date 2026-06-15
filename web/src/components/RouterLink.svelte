<script lang="ts">
  import { push } from 'svelte-spa-router';
  
  let { href, class: classStr = '' }: { href: string; class?: string } = $props();
  
  function handleClick(e: MouseEvent): void {
    e.preventDefault();
    push(href);
  }
</script>

<a {href} class={classStr} onclick={handleClick}>
  <slot></slot>
</a>
