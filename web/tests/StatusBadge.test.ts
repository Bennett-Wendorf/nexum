import { describe, it, expect } from 'vitest';
import { getPlanStatusColor, getTaskStatusColor, kanbanColumns, kanbanColumnLabels } from '../src/lib/statusColors';

describe('Status Colors', () => {
  describe('getPlanStatusColor', () => {
    it('returns colors for all plan statuses', () => {
      const statuses = ['draft', 'queued', 'planning', 'reviewing', 'approved', 'complete', 'rejected'];
      for (const status of statuses) {
        const [bg, text] = getPlanStatusColor(status);
        expect(bg).toBeTruthy();
        expect(text).toBeTruthy();
      }
    });

    it('returns fallback for unknown status', () => {
      const [bg, text] = getPlanStatusColor('unknown');
      expect(bg).toBe('bg-border-default text-text-muted');
    });
  });

  describe('getTaskStatusColor', () => {
    it('returns colors for all task statuses', () => {
      const statuses = ['backlog', 'queued', 'running', 'reviewing', 'waiting-manual-review', 'merge-queue', 'abandoned', 'completed'];
      for (const status of statuses) {
        const [bg, text] = getTaskStatusColor(status);
        expect(bg).toBeTruthy();
        expect(text).toBeTruthy();
      }
    });
  });

  describe('kanbanColumns', () => {
    it('has correct order', () => {
      expect(kanbanColumns[0]).toBe('backlog');
      expect(kanbanColumns[kanbanColumns.length - 1]).toBe('abandoned');
      expect(kanbanColumns.length).toBe(8);
    });
  });

  describe('kanbanColumnLabels', () => {
    it('has human-readable labels', () => {
      expect(kanbanColumnLabels['waiting-manual-review']).toBe('Manual Review');
      expect(kanbanColumnLabels['merge-queue']).toBe('Merge Queue');
    });
  });
});
