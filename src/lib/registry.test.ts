import { describe, it, expect, vi } from 'vitest';
import { tools, categories } from './registry';
import canonical from '../../catalog/tools.json';
import nativeIcons from '../../catalog/icons.json';
import originalIds from '../../catalog/v01-tool-ids.json';
describe('shipped tool catalogue', () => {
  it('matches all canonical metadata and native icon bindings in catalog order', () => {
    expect(canonical.tools).toHaveLength(56);
    expect(originalIds).toHaveLength(49);
    expect(originalIds.every(id=>canonical.tools.some(tool=>tool.id===id))).toBe(true);
    for (const entry of [...canonical.categories, ...canonical.tools]) {
      expect(Object.hasOwn(nativeIcons.icons, entry.icon), entry.id).toBe(true);
    }
    expect(tools.map(tool => ({id:tool.id,name:tool.name,description:tool.description,category:tool.category,keywords:tool.keywords,platforms:tool.platforms,phase:tool.phase,status:tool.status}))).toEqual(canonical.tools.map(({icon: _icon,note: _note,...metadata})=>metadata));
    expect(categories.map(({id,label})=>({id,label}))).toEqual(canonical.categories.map(({id,label})=>({id,label})));
  });
  it('reads changed metadata and icons from the canonical JSON rather than adapter literals', async () => {
    const fixture=structuredClone(canonical);
    Object.assign(fixture.tools[0],{name:'Canonical fixture',description:'Changed canonical description',platforms:['linux'],keywords:['canonical-fixture'],category:'files',icon:'archive'});
    Object.assign(fixture.categories[0],{label:'Canonical category',icon:'cpu'});
    vi.doMock('../../catalog/tools.json',()=>({default:fixture}));vi.resetModules();
    try {
      const registry=await import('./registry');
      const tool=registry.tools.find(tool=>tool.id===fixture.tools[0].id)!;
      expect(tool.name).toBe('Canonical fixture');expect(tool.description).toBe('Changed canonical description');expect(tool.platforms).toEqual(['linux']);expect(tool.keywords).toEqual(['canonical-fixture']);expect(tool.category).toBe('files');
      expect(tool.icon).toBe((await import('@lucide/svelte/icons/archive')).default);
      expect(registry.categories[0].label).toBe('Canonical category');expect(registry.categories[0].icon).toBe((await import('@lucide/svelte/icons/cpu')).default);
    } finally {vi.doUnmock('../../catalog/tools.json');vi.resetModules();}
  });
  it('has unique, implemented views for the base and expanded catalogue', async () => {
    expect(tools.filter(tool=>tool.phase<5)).toHaveLength(20);
    expect(tools).toHaveLength(56);
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
