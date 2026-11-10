// @vitest-environment jsdom
import React, { act } from 'react';
import { createRoot } from 'react-dom/client';
import { expect, it, vi } from 'vitest';
vi.mock('reactflow', async (importOriginal) => ({ ...(await importOriginal<typeof import('reactflow')>()), Handle: () => <span data-testid="handle" /> }));
it('opens one floating node card by focus, hover and touch button without covering handles', async () => {
  vi.stubGlobal('IS_REACT_ACT_ENVIRONMENT', true);
  const { FlowNode } = await import('./NodeGraphManager');
  const container = document.createElement('div'); document.body.appendChild(container); const root = createRoot(container);
  const entry = { path: 'isos/eos_hardware/base/drive', node: { id: 'drive', node_type: 'isos.eos_hardware.base@v0.2', label: 'Drive', description: 'Instance notes' }, definition: { id: 'isos.eos_hardware.base@v0.2', core: 'isos', layer: 'eos_hardware', group: 'base', kind: 'base', lifecycle: 'adapter', description: 'Catalog notes', capabilities: [], settings: [], inputs: [{ name: 'command', data_type: 'base_movement', required: true, transport: 'in_process', delivery_mode: 'message', contract: { owner: 'eos', id: 'base-movement', version: '1.0' } }], outputs: [] } };
  const props = { id: entry.path, type: 'profile', data: { entry, error: false, mirrored: false, lang: 'en', liveAllowed: false, liveRunId: null, dataTypes: [{ id: 'base_movement', version: '1.0', purpose: '', media_type: 'application/json', codec: 'json_v1', max_payload_bytes: 1024, schema: { properties: { x_m_s: { type: 'number', 'x-unit': 'm/s' } } } }] }, selected: false } as unknown as Parameters<typeof FlowNode>[0];
  await act(async () => root.render(<FlowNode {...props} />));
  const node = container.querySelector('.node-graph-card') as HTMLElement;
  await act(async () => node.dispatchEvent(new FocusEvent('focusin', { bubbles: true })));
  expect(document.body.querySelector('.node-graph-info-card')?.textContent).toContain('Catalog notes');
  expect(document.body.querySelector('.node-graph-info-card')?.textContent).toContain('Instance notes');
  expect(document.body.querySelector('.node-graph-info-card')?.textContent).toContain('m/s');
  expect(document.body.querySelector('.node-graph-info-card')?.textContent).toContain('eos/base-movement@1.0');
  expect(document.body.querySelector('.node-graph-info-card')?.textContent).toContain('json_v1');
  expect(document.body.querySelector('.node-graph-info-card')?.textContent).toContain('x_m_s');
  expect(container.querySelector('[data-testid="handle"]')).not.toBeNull();
  expect(container.querySelector('.node-graph-card__details')).toBeNull();
  await act(async () => { (document.body.querySelector('.node-graph-info-card button') as HTMLButtonElement).click(); });
  expect(document.body.querySelector('.node-graph-info-card')).toBeNull();
  await act(async () => { node.dispatchEvent(new Event('pointerdown', { bubbles: true })); node.dispatchEvent(new FocusEvent('focusin', { bubbles: true })); node.dispatchEvent(new Event('pointerup', { bubbles: true })); });
  expect(document.body.querySelector('.node-graph-info-card')).toBeNull();
  await act(async () => node.dispatchEvent(new MouseEvent('mouseover', { bubbles: true, clientX: 790, clientY: 590 })));
  expect(document.body.querySelector('.node-graph-info-card')).toBeNull();
  await act(async () => { await new Promise((resolve) => setTimeout(resolve, 580)); });
  expect(document.body.querySelector('.node-graph-info-card')).not.toBeNull();
  await act(async () => node.dispatchEvent(new KeyboardEvent('keydown', { bubbles: true, key: 'Escape' })));
  expect(document.body.querySelector('.node-graph-info-card')).toBeNull();
  await act(async () => (container.querySelector('.node-graph-card__info') as HTMLButtonElement).click());
  expect(document.body.querySelector('.node-graph-info-card')).not.toBeNull();
  await act(async () => root.unmount()); container.remove(); vi.unstubAllGlobals();
});

it('labels a health-only node without creating graph ports', async () => {
  vi.stubGlobal('IS_REACT_ACT_ENVIRONMENT', true);
  const { FlowNode } = await import('./NodeGraphManager');
  const container = document.createElement('div'); document.body.appendChild(container); const root = createRoot(container);
  const entry = { path: 'isos/eos_hardware/base/drive', node: { id: 'drive', node_type: 'isos.eos_hardware.health@v0.2', label: 'Drive', description: '' }, definition: { id: 'isos.eos_hardware.base@v0.2', core: 'isos', layer: 'eos_hardware', group: 'base', kind: 'base', lifecycle: 'adapter', capabilities: [], settings: [], inputs: [], outputs: [] } };
  await act(async () => root.render(<FlowNode {...({ id: entry.path, type: 'profile', data: { entry, error: false, mirrored: false, lang: 'en', dataTypes: [] }, selected: false } as unknown as Parameters<typeof FlowNode>[0])} />));
  expect(container.textContent).toContain('No graph ports · owner health');
  expect(container.querySelector('[data-testid="handle"]')).toBeNull();
  await act(async () => root.unmount()); container.remove(); vi.unstubAllGlobals();
});

it('shows graph placement and a catalog error when a definition is unavailable', async () => {
  vi.stubGlobal('IS_REACT_ACT_ENVIRONMENT', true);
  const { FlowNode } = await import('./NodeGraphManager');
  const container = document.createElement('div'); document.body.appendChild(container); const root = createRoot(container);
  const entry = { path: 'ei/cognition/context/builder', node: { id: 'builder', node_type: 'ei.cognition.context.health@v0.2', label: 'EI cognition', description: '' }, definition: undefined };
  await act(async () => root.render(<FlowNode {...({ id: entry.path, type: 'profile', data: { entry, error: false, mirrored: false, lang: 'en', dataTypes: [] }, selected: false } as unknown as Parameters<typeof FlowNode>[0])} />));
  expect(container.textContent).toContain('cognition / context');
  expect(container.textContent).toContain('Catalog type unavailable; ports cannot be shown');
  expect(container.querySelector('[data-testid="handle"]')).toBeNull();
  await act(async () => root.unmount()); container.remove(); vi.unstubAllGlobals();
});

it('opens on-node settings and keeps default values out of saved overrides', async () => {
  vi.stubGlobal('IS_REACT_ACT_ENVIRONMENT', true);
  const { FlowNode } = await import('./NodeGraphManager');
  const container = document.createElement('div'); document.body.appendChild(container); const root = createRoot(container);
  const change = vi.fn();
  const entry = { path: 'isos/ui/button/trigger', node: { id: 'trigger', node_type: 'isos.ui.button.trigger@v0.2', label: 'Trigger', description: '', settings: {} as Record<string, unknown> }, definition: { id: 'isos.ui.button.trigger@v0.2', core: 'isos', layer: 'ui', group: 'button', kind: 'software', lifecycle: 'internal', capabilities: [], settings: [{ name: 'label', data_type: 'string', required: true }, { name: 'timeout_ms', data_type: 'positive_integer', required: false, default: 1000 }], inputs: [], outputs: [] } };
  await act(async () => root.render(<FlowNode {...({ id: entry.path, type: 'profile', data: { entry, error: false, mirrored: false, lang: 'en', dataTypes: [], onSettingsChange: change }, selected: false } as unknown as Parameters<typeof FlowNode>[0])} />));
  await act(async () => (container.querySelector('.node-graph-card__actions button') as HTMLButtonElement).click());
  const card = document.body.querySelector('.node-graph-config-card') as HTMLElement;
  expect(document.activeElement).toBe(card.querySelector('.node-graph-config-card__close'));
  expect(card.textContent).toContain('Basic');
  expect(card.textContent).toContain('Advanced');
  expect(card.textContent).toContain('label');
  expect((card.querySelector('details') as HTMLDetailsElement).open).toBe(false);
  await act(async () => (card.querySelector('summary') as HTMLElement).click());
  expect(card.textContent).toContain('Default: 1000');
  expect(container.querySelector('.node-graph-card__actions button:last-child')?.hasAttribute('disabled')).toBe(true);
  expect(change).not.toHaveBeenCalled();
  await act(async () => (card.querySelector('.node-graph-config-card__close') as HTMLButtonElement).click());
  expect(document.activeElement).toBe(container.querySelector('.node-graph-card__actions button'));
  await act(async () => root.unmount()); container.remove(); vi.unstubAllGlobals();
});

it('shows required adapter fields in Basic before Advanced is opened', async () => {
  vi.stubGlobal('IS_REACT_ACT_ENVIRONMENT', true);
  const { FlowNode } = await import('./NodeGraphManager');
  const container = document.createElement('div'); document.body.appendChild(container); const root = createRoot(container);
  const entry = { path: 'eos/hardware/mock/arm', node: { id: 'arm', node_type: 'eos.hardware.mock.arm@v0.2', label: 'Arm', description: '', adapter: { type: 'mock', settings: {} } }, definition: { id: 'eos.hardware.mock.arm@v0.2', core: 'eos', layer: 'hardware', group: 'mock', kind: 'manipulator', lifecycle: 'adapter', capabilities: [], settings: [], inputs: [], outputs: [] } };
  await act(async () => root.render(<FlowNode {...({ id: entry.path, type: 'profile', data: { entry, error: false, mirrored: false, lang: 'en', dataTypes: [] }, selected: false } as unknown as Parameters<typeof FlowNode>[0])} />));
  await act(async () => (container.querySelector('.node-graph-card__actions button') as HTMLButtonElement).click());
  const card = document.body.querySelector('.node-graph-config-card') as HTMLElement;
  expect(card.querySelector('details')?.open).toBe(false);
  expect(card.querySelector('input')?.closest('label')?.textContent).toContain('backend');
  expect(card.textContent).not.toContain('Nothing required; this node is ready.');
  await act(async () => root.unmount()); container.remove(); vi.unstubAllGlobals();
});
