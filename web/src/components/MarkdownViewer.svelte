<script>
  import { renderMarkdown } from '../lib/markdown.js';
  
  let { content = '' } = $props();
  
  $derived(html = renderMarkdown(content));
</script>

<div class="prose prose-sm max-w-none">
  {@html html}
</div>

<style global>
  .prose h1 { font-size: 1.5rem; font-weight: 700; margin-bottom: 0.5rem; }
  .prose h2 { font-size: 1.25rem; font-weight: 600; margin-bottom: 0.5rem; }
  .prose p { margin-bottom: 0.75rem; }
  .prose ul { list-style-type: disc; padding-left: 1.5rem; }
  .prose ol { list-style-type: decimal; padding-left: 1.5rem; }
  .prose code { background-color: #f3f4f6; padding: 0.125rem 0.25rem; border-radius: 0.25rem; }
  .prose pre { background-color: #1f2937; color: #f9fafb; padding: 1rem; border-radius: 0.5rem; overflow-x: auto; }
  .prose a { color: #3b82f6; text-decoration: underline; }
</style>
