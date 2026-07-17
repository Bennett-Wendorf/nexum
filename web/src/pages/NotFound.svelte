<script lang="ts">
  import RouterLink from '../components/RouterLink.svelte';
  // Svelte 5 404 page
</script>

<div class="container mx-auto px-4 py-8 text-center">
  <h1 class="text-3xl font-bold text-text-primary">404</h1>
  <p class="text-text-muted mt-2">Page not found</p>
  <RouterLink href="/" class="inline-block mt-4 px-4 py-2 bg-btn-green text-white rounded-md text-sm font-medium hover:bg-btn-green-hover transition-colors">
    Go Home
  </RouterLink>
</div>
