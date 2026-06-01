<script>
  import Router from 'svelte-spa-router';
  import { routes } from './routes/index.js';
</script>

<main class="min-h-screen bg-gray-50">
  <Router {routes} />
</main>
