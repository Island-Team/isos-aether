// @vitest-environment jsdom
import React, { act } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, expect, it, vi } from 'vitest';
import { GraphDependencyRepair } from './GraphDependencyRepair';

const mocks = vi.hoisted(() => ({ preview: vi.fn(), install: vi.fn(), source: vi.fn() }));
vi.mock('@/services/isos/node-graph', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/services/isos/node-graph')>()),
  previewPackageDownload: mocks.preview,
  installBackend: mocks.install,
  installSourceBackend: mocks.source,
}));

const graph = { version: '0.2.1', robot: { id: 'test', name: 'Test' }, cores: {
  isos: { layers: {} }, ei: { layers: {} }, eos: { layers: { hardware: { groups: { camera: { nodes: {
    front: { id: 'front', node_type: 'eos.hardware.camera.device@v0.2', label: 'Front', description: '', package: { id: 'island/mujoco-sim', version: '0.2.7' } },
  } } } } } },
} };
const readiness = { cores: [{ core: 'eos', path: null, version: null, compatible: true, configured: true, graph_configured: false, running: false, ready: false, reason: 'catalog unavailable: Reinstall or relink island/mujoco-sim@0.2.7' }] };

afterEach(() => { vi.resetAllMocks(); vi.unstubAllGlobals(); document.body.innerHTML = ''; });

async function mount() {
  vi.stubGlobal('IS_REACT_ACT_ENVIRONMENT', true);
  const host = document.createElement('div'); document.body.appendChild(host);
  const root = createRoot(host);
  const onInstalled = vi.fn(); const onRetry = vi.fn();
  await act(async () => root.render(<GraphDependencyRepair graph={graph} installed={[]} readiness={readiness} lang="en" onInstalled={onInstalled} onRetry={onRetry} />));
  return { host, root, onInstalled, onRetry };
}

it('shows the exact dependency and installs only the approved release', async () => {
  const { host, root, onInstalled } = await mount();
  expect(host.textContent).toContain('catalog unavailable: Reinstall or relink island/mujoco-sim@0.2.7');
  mocks.preview.mockResolvedValue({ preview: { id: 'island/mujoco-sim', version: '0.2.7', publisher: 'Island', source_url: null, url: 'https://example.test/release', size_bytes: 42, sha256: 'a'.repeat(64), trust: 'verified', approval: 'digest' } });
  mocks.install.mockResolvedValue({ verified: true });
  const confirm = vi.fn(() => true); vi.stubGlobal('confirm', confirm);
  await act(async () => host.querySelector<HTMLButtonElement>('button')!.click());
  expect(confirm).toHaveBeenCalledOnce();
  expect(mocks.install).toHaveBeenCalledWith('island/mujoco-sim', '0.2.7', 'digest');
  expect(onInstalled).toHaveBeenCalledOnce();
  await act(async () => root.unmount());
});

it('builds a selected local checkout only after confirmation', async () => {
  const { host, root, onInstalled } = await mount();
  const input = host.querySelector('input')!;
  await act(async () => { Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')!.set!.call(input, '/tmp/mujoco-source'); input.dispatchEvent(new Event('input', { bubbles: true })); });
  const confirm = vi.fn(() => true); vi.stubGlobal('confirm', confirm);
  mocks.source.mockResolvedValue({ verified: true });
  await act(async () => [...host.querySelectorAll('button')].find((button) => button.textContent === 'Build and install from source (dev mode)')!.click());
  expect(mocks.source).toHaveBeenCalledWith('island/mujoco-sim', '0.2.7', '/tmp/mujoco-source');
  expect(onInstalled).toHaveBeenCalledOnce();
  await act(async () => root.unmount());
});

it('does not download when release approval is declined', async () => {
  const { host, root, onInstalled } = await mount();
  mocks.preview.mockResolvedValue({ preview: { id: 'island/mujoco-sim', version: '0.2.7', publisher: 'Island', source_url: null, url: 'https://example.test/release', size_bytes: 42, sha256: 'a'.repeat(64), trust: 'verified', approval: 'digest' } });
  vi.stubGlobal('confirm', vi.fn(() => false));
  await act(async () => host.querySelector<HTMLButtonElement>('button')!.click());
  expect(mocks.install).not.toHaveBeenCalled();
  expect(onInstalled).not.toHaveBeenCalled();
  await act(async () => root.unmount());
});
