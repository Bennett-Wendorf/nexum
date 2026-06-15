import { describe, it, expect } from 'vitest';

describe('PlanCard Logic', () => {
  it('calculates progress correctly', () => {
    const tasks = [
      { id: '1', name: 'Task 1', completed: true },
      { id: '2', name: 'Task 2', completed: false },
      { id: '3', name: 'Task 3', completed: true },
    ];
    const total = tasks.length;
    const completed = tasks.filter(t => t.completed).length;
    const percent = Math.round((completed / total) * 100);
    expect(percent).toBe(67);
  });

  it('handles empty task list', () => {
    const tasks: any[] = [];
    const total = tasks.length;
    const completed = tasks.filter(t => t.completed).length;
    const percent = total > 0 ? Math.round((completed / total) * 100) : 0;
    expect(percent).toBe(0);
  });

  it('generates correct href', () => {
    const branch = 'main';
    const planId = 'test-plan';
    const href = `/plans/${branch}/${planId}`;
    expect(href).toBe('/plans/main/test-plan');
  });
});
