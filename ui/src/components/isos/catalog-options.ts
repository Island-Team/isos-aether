import type { NodeGraph, NodeOptions, NodeTypeCatalog } from '@/services/isos/node-graph';
import { graphNodes, type GraphNode } from './node-graph-edits';
import { settingOptionGroup, unavailableSelections } from './CatalogSelector';

export interface OptionSnapshot { draft: string; options: NodeOptions }

/** Preserve prior owner rows for context; current catalog families replace stale family rows. */
export function displayNodeOptions(current: NodeOptions | null, previous: NodeOptions | null, preview: NodeOptions | null): NodeOptions | null {
  if (current) return current;
  if (!preview) return previous;
  return { ...(previous || preview), setting_options: [
    ...(previous?.setting_options || []).filter((group) => group.setting !== 'adapter.type'),
    ...(preview.setting_options || []),
  ] };
}

/** Let an incomplete draft select a verified family while owner setup waits for required fields. */
export function draftAdapterOptions(graph: NodeGraph, catalog: NodeTypeCatalog): NodeOptions {
  const groups = graphNodes(graph, catalog.types).filter((entry) => entry.definition?.lifecycle === 'adapter').map((entry) => ({
    node_path: entry.path,
    setting: 'adapter.type',
    selected: entry.node.adapter?.type || null,
    options: (catalog.adapter_families || []).filter((family) =>
      family.device_kinds.includes(entry.definition!.kind) &&
      (family.package?.id || '') === (entry.node.package?.id || '') &&
      (family.package?.version || '') === (entry.node.package?.version || '')
    ).map((family) => ({
      value: family.id, label: family.id, description: family.description,
      available: family.compiled || Boolean(family.loadable),
      reason_code: family.compiled || family.loadable ? null : 'adapter_unavailable',
      reason: family.compiled || family.loadable ? null : family.reason || 'EOS adapter implementation is unavailable',
    })),
  }));
  return { adapter_families: [], primitive_bindings: [], targets: [], setting_options: groups };
}

export function catalogErrorIsStable(blocked: boolean, draft: string, snapshot: OptionSnapshot | null, failedDraft: string): boolean {
  return blocked && (snapshot?.draft === draft || failedDraft === draft);
}

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
  if (graphVersion !== '0.2.1') return false;
  const selectors = entries.flatMap((entry) => (entry.definition?.settings || []).filter((field) => field.selector).map((field) => ({ entry, field })));
  if (selectors.some(({ field }) => field.selector?.provider === 'ui')) return true;
  const remote = selectors.filter(({ field }) => field.selector?.provider !== 'ui');
  const adapters = entries.filter((entry) => entry.definition?.lifecycle === 'adapter');
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
      for (const group of options.setting_options || []) {
        if (group.node_path === entry.path && group.setting.startsWith('adapter.settings.')) {
          selected[group.setting] = settings[group.setting.slice('adapter.settings.'.length)];
        }
      }
      if (entry.definition?.kind === 'camera' && entry.node.package && !settings.device_id) return true;
      return Object.entries(selected).some(([name, value]) => {
        const group = settingOptionGroup(options, entry.path, name);
        return !group || typeof value !== 'string' || !value.trim() || !group.options.some((option) => option.value === value && option.available);
      });
    });
}
