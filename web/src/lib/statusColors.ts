// Plan statuses: draft, queued, planning, reviewing, approved, complete, rejected
export const planStatusColors: Record<string, string> = {
  draft: 'text-text-muted',
  queued: 'text-accent-blue',
  planning: 'text-accent-purple',
  reviewing: 'text-accent-yellow',
  approved: 'text-accent-green',
  complete: 'text-accent-green',
  rejected: 'text-accent-red',
};

export const planStatusBgColors: Record<string, string> = {
  draft: 'bg-border-default text-text-muted',
  queued: 'bg-accent-blue-subtle text-accent-blue',
  planning: 'bg-accent-purple-subtle text-accent-purple',
  reviewing: 'bg-accent-yellow-subtle text-accent-yellow',
  approved: 'bg-accent-green-subtle text-accent-green',
  complete: 'bg-accent-green-subtle text-accent-green',
  rejected: 'bg-accent-red-subtle text-accent-red',
};

// Task statuses: backlog, queued, running, reviewing, waiting-manual-review, merge-queue, abandoned, completed
export const taskStatusColors: Record<string, string> = {
  backlog: 'text-text-muted',
  queued: 'text-accent-blue',
  running: 'text-accent-blue',
  reviewing: 'text-accent-yellow',
  'waiting-manual-review': 'text-accent-yellow',
  'merge-queue': 'text-accent-purple',
  abandoned: 'text-accent-red',
  completed: 'text-accent-green',
};

export const taskStatusBgColors: Record<string, string> = {
  backlog: 'bg-border-default text-text-muted',
  queued: 'bg-accent-blue-subtle text-accent-blue',
  running: 'bg-accent-blue-subtle text-accent-blue',
  reviewing: 'bg-accent-yellow-subtle text-accent-yellow',
  'waiting-manual-review': 'bg-accent-yellow-subtle text-accent-yellow',
  'merge-queue': 'bg-accent-purple-subtle text-accent-purple',
  abandoned: 'bg-accent-red-subtle text-accent-red',
  completed: 'bg-accent-green-subtle text-accent-green',
};

// Kanban columns in display order
export const kanbanColumns = [
  'backlog',
  'queued',
  'running',
  'reviewing',
  'waiting-manual-review',
  'merge-queue',
  'completed',
  'abandoned',
] as const;

// Human-readable labels for kanban columns
export const kanbanColumnLabels: Record<string, string> = {
  backlog: 'Backlog',
  queued: 'Queued',
  running: 'Running',
  reviewing: 'Reviewing',
  'waiting-manual-review': 'Manual Review',
  'merge-queue': 'Merge Queue',
  completed: 'Completed',
  abandoned: 'Abandoned',
  // Plan status labels
  draft: 'Draft',
  planning: 'Planning',
  approved: 'Approved',
  complete: 'Complete',
  rejected: 'Rejected',
};

/**
 * Get the color class for a plan status badge.
 */
export function getPlanStatusColor(status: string): [string, string] {
  const bg = planStatusBgColors[status] ?? 'bg-border-default text-text-muted';
  const text = planStatusColors[status] ?? 'text-text-muted';
  return [bg, text];
}

/**
 * Get the color class for a task status badge.
 */
export function getTaskStatusColor(status: string): [string, string] {
  const bg = taskStatusBgColors[status] ?? 'bg-border-default text-text-muted';
  const text = taskStatusColors[status] ?? 'text-text-muted';
  return [bg, text];
}
