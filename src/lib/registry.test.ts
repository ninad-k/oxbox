import { describe, it, expect } from 'vitest';
import { tools, categories } from './registry';
describe('shipped tool catalogue', () => {
  it('has unique, implemented views for the base and expanded catalogue', async () => {
    expect(tools.filter(tool=>tool.phase<5)).toHaveLength(20);
    expect(tools).toHaveLength(48);
    expect(new Set(tools.map(tool => tool.id)).size).toBe(tools.length);
    for (const tool of tools) {
      expect(tool.status, tool.id).toBe('ready');
      expect(categories.some(category => category.id === tool.category), tool.id).toBe(true);
      expect(tool.platforms.length, tool.id).toBeGreaterThan(0);
      expect(tool.load, tool.id).toBeTypeOf('function');
      expect((await tool.load!()).default, tool.id).toBeTypeOf('function');
    }
  });
});
