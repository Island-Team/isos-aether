import type { NodeGraph, NodeOptions } from '@/services/isos/node-graph';
import type { GraphNode } from './node-graph-edits';
import { settingOptionGroup, unavailableSelections } from './CatalogSelector';

export interface OptionSnapshot { draft: string; options: NodeOptions }

export function optionResponseIsCurrent(requestId: number, latestRequestId: number, requestedDraft: string, currentDraft: string): boolean {
  return requestId === latestRequestId && requestedDraft === currentDraft;
}

export function catalogReloadIsCurrent(requestId: number, latestRequestId: number): boolean {
  return requestId === latestRequestId;
}

export function catalogReloadGraph(savedRevision: string, currentRevision: string, currentGraph: NodeGraph | null, savedGraph: NodeGraph): NodeGraph | null {
  return savedRevision === currentRevision ? currentGraph || savedGraph : null;
}

export function catalogSelectionBlocks(graphVersion: string | undefined, entries: GraphNode[], options: NodeOptions | null): boolean {
  if (graphVersion !== '0.2.0') return false;
  const selectors = entries.flatMap((entry) => (entry.definition?.settings || []).filter((field) => field.selector).map((field) => ({ entry, field })));
  if (selectors.some(({ field }) => field.selector?.provider === 'ui')) return true;
  const remote = selectors.filter(({ field }) => field.selector?.provider !== 'ui');
  const adapters = entries.filter((entry) => entry.definition?.lifecycle === 'adapter' && entry.node.adapter);
  if (!remote.length && !adapters.length) return !!unavailableSelections(options).length;
  if (!options || unavailableSelections(options).length) return true;
  return remote.some(({ entry, field }) => {
    const group = settingOptionGroup(options, entry.path, field.name);
    const effective = entry.node.settings?.[field.name] ?? field.default;
    if (!group) return true;
    if (field.required && !String(effective ?? '').trim()) return true;
    return field.selector?.provider === 'owner' && effective !== undefined && (typeof effective !== 'string' || !group.options.some((option) => option.value === effective && option.available));
  })
    || adapters.some((entry) => {
      const settings = entry.node.adapter?.settings || {};
      const selected: Record<string, unknown> = {
        'adapter.type': entry.node.adapter?.type,
      };
      if (entry.node.adapter?.type === 'mujoco-v2' || settingOptionGroup(options, entry.path, 'adapter.settings.device_id')) selected['adapter.settings.device_id'] = settings.device_id;
      if (entry.node.adapter?.type === 'mujoco-v2' || settingOptionGroup(options, entry.path, 'adapter.settings.simulator_node')) selected['adapter.settings.simulator_node'] = settings.simulator_node;
      return Object.entries(selected).some(([name, value]) => {
        const group = settingOptionGroup(options, entry.path, name);
        return !group || typeof value !== 'string' || !value.trim() || !group.options.some((option) => option.value === value && option.available);
      });
    });
}
