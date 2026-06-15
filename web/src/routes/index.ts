import type { RouteDefinition } from 'svelte-spa-router';
import PlanList from '../pages/PlanList.svelte';
import PlanDetail from '../pages/PlanDetail.svelte';
import TaskList from '../pages/TaskList.svelte';
import TaskDetail from '../pages/TaskDetail.svelte';
import NotFound from '../pages/NotFound.svelte';

export const routes: RouteDefinition = {
  '/': PlanList,
  '/plans': PlanList,
  '/plans/:branch/:planId': PlanDetail,
  '/plans/:branch/:planId/tasks': TaskList,
  '/plans/:branch/:planId/tasks/:taskId': TaskDetail,
  '*': NotFound,
};
