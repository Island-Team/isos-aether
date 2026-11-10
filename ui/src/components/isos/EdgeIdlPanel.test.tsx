import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vitest';
import { EdgeIdlPanel } from './NodeGraphManager';
import type { GraphNode } from './node-graph-edits';
import type { PortDefinition } from '@/services/isos/node-graph';

const port = (delivery_mode: string, direction: 'input' | 'output'): PortDefinition => ({ name: direction === 'input' ? 'incoming' : 'outgoing', data_type: 'status@v1', transport: delivery_mode === 'declaration' ? 'none' : 'unix_socket', delivery_mode, required: false, ...(delivery_mode === 'declaration' ? {} : { contract: { owner: 'eos', id: 'status', version: '1.0' } }) });
const entries = (mode: string): GraphNode[] => [
  { path: 'a', node: { id: 'a', node_type: 'sample.a@v0.2', label: '', description: '' }, definition: { id: 'sample.a@v0.2', core: 'isos', layer: 'x', group: 'y', kind: 'software', lifecycle: 'internal', capabilities: [], settings: [], inputs: [], outputs: [port(mode, 'output')] } },
  { path: 'b', node: { id: 'b', node_type: 'sample.b@v0.2', label: '', description: '' }, definition: { id: 'sample.b@v0.2', core: 'isos', layer: 'x', group: 'y', kind: 'software', lifecycle: 'internal', capabilities: [], settings: [], inputs: [port(mode, 'input')], outputs: [] } },
];
const edge = { id: 'a-b', source: 'a', sourceHandle: 'out:outgoing', target: 'b', targetHandle: 'in:incoming' };
const panel = (mode: string, lang: 'en' | 'cn') => renderToStaticMarkup(<EdgeIdlPanel edge={edge} entries={entries(mode)} dataTypes={[{ id: 'status@v1', version: '1', purpose: 'status', codec: 'json', media_type: 'application/json', max_payload_bytes: 64, schema: { type: 'object', properties: { state: { type: 'string' } } } }]} diagnostics={[]} validated processStatus={null} lang={lang} />);

describe('edge details', () => {
  it('labels declarations without claiming delivery', () => { expect(panel('declaration', 'en')).toContain('Static topology link'); expect(panel('declaration', 'cn')).toContain('静态拓扑连接'); });
  it('shows live channel, protocol and payload', () => { const html = panel('event', 'en'); expect(html).toContain('Live data connection'); expect(html).toContain('unix_socket'); expect(html).toContain('eos/status@1.0'); expect(html).toContain('Fields'); expect(html).toContain('state'); });
});
