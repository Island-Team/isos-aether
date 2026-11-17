// @vitest-environment jsdom
import React, { act } from 'react';
import { createRoot } from 'react-dom/client';
import { describe, expect, it } from 'vitest';
import { CatalogBrowser, packageAccent, TypeTree } from './CatalogBrowser';
import { PackageLibrary } from './PackageLibrary';
import { catalogTypeKey, type InstalledPackage, type NodeTypeDefinition } from '../../services/isos/node-graph';

const definition = (core: string, pkg?: { id: string; version: string }, group = 'camera'): NodeTypeDefinition => ({
  id: `${core}.hardware.${group}.rgb@v0.2`, core, layer: 'hardware', group,
  kind: 'camera', lifecycle: 'adapter', capabilities: [], settings: [], inputs: [], outputs: [],
  ...(pkg ? { package: pkg } : {}),
});
const receipt = (id: string, version: string, configurable = false): InstalledPackage => ({ schema_version: 2, id, version, source_commit: null, asset_sha256: null, local_link: null, configurable });

function mount(element: React.ReactElement) {
  (globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
  const host = document.createElement('div'); document.body.appendChild(host);
  const root = createRoot(host); act(() => root.render(element));
  return { host, cleanup: () => { act(() => root.unmount()); host.remove(); } };
}

describe('file-style libraries', () => {
  it('nests core, layer, and group; adds package folders only for overlapping package sources', () => {
    const builtIn = definition('eos');
    const first = definition('eos', { id: 'vendor/driver', version: '0.2.1' });
    const second = definition('eos', { id: 'other/driver', version: '0.2.1' });
    const ei = definition('ei', { id: 'vendor/driver', version: '0.2.1' }, 'sensor');
    const legacy = { ...first, id: 'eos.process.driver.simulator@v0.2', layer: 'process', implementation: { kind: 'managed_process' as const } };
    let selected = '';
    const view = mount(<CatalogBrowser types={[builtIn, first, second, ei, legacy]} value="" lang="en" onChange={(key) => { selected = key; }} />);
    const folders = [...view.host.querySelectorAll('.node-library-folder > summary')].map((item) => item.textContent);
    expect(folders).toContain('eos'); expect(folders).toContain('hardware'); expect(folders).toContain('camera');
    expect(folders).toContain('vendor/driver@0.2.1'); expect(folders).toContain('other/driver@0.2.1');
    expect(view.host.textContent).not.toContain('simulator');
    expect(view.host.querySelectorAll('.node-library-folder--package')).toHaveLength(2);
    expect(view.host.querySelector('.node-library-type__mark--package')).not.toBeNull();
    expect(packageAccent('vendor/driver@0.2.1')).toBe(packageAccent('vendor/driver@0.2.1'));
    act(() => (view.host.querySelectorAll('.node-library-folder--package button')[0] as HTMLButtonElement).click());
    expect(selected).toBe(catalogTypeKey(second));
    view.cleanup();
  });

  it('shows one package directly in a group and searches across cores', () => {
    const builtIn = definition('eos');
    const ei = definition('ei', { id: 'vendor/driver', version: '0.2.1' });
    const view = mount(<CatalogBrowser types={[builtIn, ei]} value={catalogTypeKey(ei)} lang="cn" onChange={() => undefined} />);
    expect(view.host.querySelectorAll('.node-library-folder--package')).toHaveLength(0);
    const search = view.host.querySelector('input[type="search"]') as HTMLInputElement;
    act(() => { Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')?.set?.call(search, 'camera'); search.dispatchEvent(new Event('input', { bubbles: true })); });
    expect(view.host.textContent).toContain('eos'); expect(view.host.textContent).toContain('ei');
    expect(view.host.textContent).toContain('vendor/driver@0.2.1');
    expect(view.host.querySelector('button[aria-pressed="true"]')).not.toBeNull();
    view.cleanup();
  });

  it('lists installed packages with and without nodes and scopes each package tree', () => {
    const first = definition('eos', { id: 'vendor/driver', version: '0.2.1' });
    const second = definition('eos', { id: 'vendor/driver', version: '0.2.2' });
    const installed = [receipt('vendor/driver', '0.2.1', true), receipt('vendor/driver', '0.2.2'), receipt('vendor/empty', '1.0.0')];
    let selectedPin = ''; let selectedType = ''; let refreshed = 0;
    const view = mount(<PackageLibrary installed={installed} types={[first, second]} selectedPin="" selectedType="" lang="en" onSelectPackage={(pin) => { selectedPin = pin; return true; }} onSelectType={(key) => { selectedType = key; }} onRefresh={() => { refreshed += 1; }} />);
    expect(view.host.querySelectorAll('.node-package-library__entry')).toHaveLength(3);
    expect(view.host.textContent).toContain('This package contributes no node types.');
    act(() => (view.host.querySelectorAll('.node-package-library__entry')[1].querySelector(':scope > summary') as HTMLElement).click());
    expect(selectedPin).toBe('vendor/driver@0.2.2');
    act(() => (view.host.querySelectorAll('.node-package-library__entry')[1].querySelector('button.node-library-type') as HTMLButtonElement).click());
    expect(selectedType).toBe(catalogTypeKey(second));
    expect(view.host.querySelectorAll('.node-package-library__entry')[0].querySelectorAll('button.node-library-type')).toHaveLength(1);
    act(() => (view.host.querySelector('button[aria-label="Refresh packages"]') as HTMLButtonElement).click());
    expect(refreshed).toBe(1);
    view.cleanup();
  });

  it('uses native disclosure controls for keyboard access', () => {
    const view = mount(<TypeTree types={[definition('eos')]} value="" lang="en" onChange={() => undefined} />);
    const summaries = view.host.querySelectorAll('summary');
    expect(summaries).toHaveLength(3);
    summaries.forEach((summary) => expect(summary.parentElement?.tagName).toBe('DETAILS'));
    expect((view.host.querySelector('button.node-library-type') as HTMLButtonElement).type).toBe('button');
    view.cleanup();
  });
});
