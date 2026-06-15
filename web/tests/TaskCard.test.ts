import { describe, it, expect } from 'vitest';

describe('TaskCard Logic', () => {
  it('generates correct href', () => {
    const branch = 'main';
    const planId = 'test-plan';
    const taskId = 'task-1';
    const href = `/plans/${branch}/${planId}/tasks/${taskId}`;
    expect(href).toBe('/plans/main/test-plan/tasks/task-1');
  });

  it('extracts agent name from task status', () => {
    const task = {
      status: {
        agent: { role: 'builder' },
      },
    };
    const agentName = task.status.agent?.role ?? null;
    expect(agentName).toBe('builder');
  });

  it('handles missing agent', () => {
    const task = {
      status: {
        agent: undefined,
      },
    };
    const agentName = task.status.agent?.role ?? null;
    expect(agentName).toBeNull();
  });
});
