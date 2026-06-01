import Home from '../pages/Home.svelte';
import NotFound from '../pages/NotFound.svelte';

export const routes = {
  '/': Home,
  '*': NotFound,
};
