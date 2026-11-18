import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vitest';
import type { NodeGraph } from '../../services/isos/node-graph';
import { GraphVision, graphVisionMonitors } from './GraphVision';

const graph = { version: '0.2.1', robot: { id: 'robot', name: 'Robot' }, cores: { isos: { layers: { ui: { groups: { vision: { nodes: { front: { id: 'front', node_type: 'isos.ui.vision.monitor@v0.2', label: 'Front', description: '' }, rear: { id: 'rear', node_type: 'isos.ui.vision.monitor@v0.2', label: 'Rear', description: '' } } } } } } } } } as NodeGraph;

describe('graph-driven Vision page', () => {
  it('shows one labeled view for each UI vision node and no page without one', () => {
    expect(graphVisionMonitors(graph).map((item) => item.path)).toEqual(['isos/ui/vision/front', 'isos/ui/vision/rear']);
    const html = renderToStaticMarkup(<GraphVision lang="en" graph={graph} revision="" status={null} visible={false} />);
    expect(html).toContain('Front'); expect(html).toContain('Rear');
    expect(GraphVision({ lang: 'en', graph: null, revision: '', status: null, visible: false })).toBeNull();
  });
});
