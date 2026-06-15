<script lang="ts">
  import Router from 'svelte-spa-router';
  import { routes } from './routes';
  import Layout from './components/Layout.svelte';
</script>

<Layout>
  <Router {routes} />
</Layout>
