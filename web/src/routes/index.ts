import type { RouteDefinition } from 'svelte-spa-router';
import Home from '../pages/Home.svelte';
import NotFound from '../pages/NotFound.svelte';

export const routes: RouteDefinition = {
  '/': Home,
  '*': NotFound,
};
