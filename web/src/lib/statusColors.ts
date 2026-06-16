export const planStatusBgColors: Record<string, string> = {
  draft: 'bg-accent-blue-subtle text-accent-blue',
  active: 'bg-accent-green-subtle text-accent-green',
  paused: 'bg-accent-yellow-subtle text-accent-yellow',
  completed: 'bg-accent-purple-subtle text-accent-purple',
  archived: 'bg-border-default text-text-muted',
};

export const taskStatusBgColors: Record<string, string> = {
  todo: 'bg-accent-blue-subtle text-accent-blue',
  in_progress: 'bg-accent-yellow-subtle text-accent-yellow',
  done: 'bg-accent-green-subtle text-accent-green',
  cancelled: 'bg-border-default text-text-muted',
};

export function getPlanStatusColor(status: string): string {
  return planStatusBgColors[status] ?? 'bg-border-default text-text-muted';
}

export function getTaskStatusColor(status: string): string {
  return taskStatusBgColors[status] ?? 'bg-border-default text-text-muted';
}

export const kanbanColumnLabels: Record<string, string> = {
  todo: 'To Do',
  in_progress: 'In Progress',
  done: 'Done',
  draft: 'Draft',
  active: 'Active',
  paused: 'Paused',
  completed: 'Completed',
  archived: 'Archived',
  cancelled: 'Cancelled',
};
