// @vitest-environment jsdom
import React, { act } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({ get: vi.fn(), validate: vi.fn(), put: vi.fn() }));
vi.mock('@/services/isos/node-graph', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/services/isos/node-graph')>()),
  getPackageConfig: mocks.get,
  validatePackageConfig: mocks.validate,
  putPackageConfig: mocks.put,
}));

afterEach(() => { vi.clearAllMocks(); vi.unstubAllGlobals(); document.body.innerHTML = ''; });

it('renders package-described fields and saves values against the exact config revision', async () => {
  vi.stubGlobal('IS_REACT_ACT_ENVIRONMENT', true);
  mocks.get.mockResolvedValue({
    id: 'vendor/robot', version: '0.2.1', deployment: 'default', revision: 'abc',
    toml: '[host]\nroot = "/opt/robot"\n', values: { host: { root: '/opt/robot' } },
    descriptor: { contract: 'isos.package-config@v1', schema_version: '0.1.0', sections: [{
      id: 'host', label: { en: 'Host', cn: '主机' }, description: { en: 'Local requirements', cn: '本机要求' },
      fields: [{ name: 'root', type: 'directory', required: true, label: { en: 'Runtime root', cn: '运行时目录' }, description: { en: 'Installed runtime', cn: '已安装运行时' } }],
    }] },
  });
  mocks.validate.mockResolvedValue({ valid: true, diagnostics: [] });
  mocks.put.mockResolvedValue({ revision: 'def', toml: '[host]\nroot = "/srv/robot"\n' });
  const { PackageConfigPanel } = await import('./PackageConfigPanel');
  const onDirtyChange = vi.fn();
  const container = document.createElement('div'); document.body.appendChild(container);
  const root = createRoot(container);
  await act(async () => root.render(<PackageConfigPanel id="vendor/robot" version="0.2.1" lang="en" onDirtyChange={onDirtyChange} />));
  const field = container.querySelector<HTMLInputElement>('input[id*="host-root"]')!;
  expect(field.value).toBe('/opt/robot');
  await act(async () => { Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')!.set!.call(field, '/srv/robot'); field.dispatchEvent(new Event('input', { bubbles: true })); });
  expect(onDirtyChange).toHaveBeenLastCalledWith(true);
  await act(async () => [...container.querySelectorAll('button')].find((button) => button.textContent === 'Save configuration')!.click());
  expect(mocks.validate).toHaveBeenCalledWith('vendor/robot', '0.2.1', { values: { host: { root: '/srv/robot' } } });
  expect(mocks.put).toHaveBeenCalledWith('vendor/robot', '0.2.1', 'abc', { values: { host: { root: '/srv/robot' } } });
  await act(async () => root.unmount());
});

it('asks before discarding edits when switching between form and TOML', async () => {
  vi.stubGlobal('IS_REACT_ACT_ENVIRONMENT', true);
  mocks.get.mockResolvedValue({
    id: 'vendor/robot', version: '0.2.1', deployment: 'default', revision: 'abc',
    toml: '[host]\nroot = "/opt/robot"\n', values: { host: { root: '/opt/robot' } },
    descriptor: { contract: 'isos.package-config@v1', schema_version: '0.1.0', sections: [{
      id: 'host', label: { en: 'Host', cn: '主机' }, description: { en: 'Local', cn: '本机' },
      fields: [{ name: 'root', type: 'directory', required: true, label: { en: 'Runtime root', cn: '运行目录' }, description: { en: 'Runtime', cn: '运行时' } }],
    }] },
  });
  const { PackageConfigPanel } = await import('./PackageConfigPanel');
  const container = document.createElement('div'); document.body.appendChild(container);
  const root = createRoot(container);
  await act(async () => root.render(<PackageConfigPanel id="vendor/robot" version="0.2.1" lang="en" />));
  const field = container.querySelector<HTMLInputElement>('input[id*="host-root"]')!;
  await act(async () => { Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')!.set!.call(field, '/srv/robot'); field.dispatchEvent(new Event('input', { bubbles: true })); });
  const tomlButton = [...container.querySelectorAll('button')].find((button) => button.textContent === 'TOML')!;
  const confirm = vi.fn().mockReturnValueOnce(false).mockReturnValueOnce(true);
  vi.stubGlobal('confirm', confirm);
  await act(async () => tomlButton.click());
  expect(container.querySelector('textarea')).toBeNull();
  expect(field.value).toBe('/srv/robot');
  await act(async () => tomlButton.click());
  expect(container.querySelector<HTMLTextAreaElement>('textarea')?.value).toBe('[host]\nroot = "/opt/robot"\n');
  expect(confirm).toHaveBeenCalledTimes(2);
  await act(async () => root.unmount());
});

it('renders verified enum options and saves the selected value', async () => {
  vi.stubGlobal('IS_REACT_ACT_ENVIRONMENT', true);
  mocks.get.mockResolvedValue({
    id: 'vendor/robot', version: '0.2.1', deployment: 'default', revision: 'abc',
    toml: '[services.native]\nscene = "pick-place"\n', values: { services: { native: { scene: 'pick-place' } } },
    descriptor: { contract: 'isos.package-config@v1', schema_version: '0.1.0', sections: [{
      id: 'services.native', label: { en: 'Native', cn: '本地' }, description: { en: 'Settings', cn: '设置' },
      fields: [{ name: 'scene', type: 'enum', required: true, label: { en: 'Scene', cn: '场景' }, description: { en: 'Verified scene', cn: '已验证场景' }, options: [{ value: 'pick-place', label: { en: 'Pick place', cn: '抓取放置' } }, { value: 'navigation', label: { en: 'Navigation', cn: '导航' } }] }],
    }] },
  });
  mocks.validate.mockResolvedValue({ valid: true, diagnostics: [] });
  mocks.put.mockResolvedValue({ revision: 'def', toml: '[services.native]\nscene = "navigation"\n' });
  const { PackageConfigPanel } = await import('./PackageConfigPanel');
  const container = document.createElement('div'); document.body.appendChild(container);
  const root = createRoot(container);
  await act(async () => root.render(<PackageConfigPanel id="vendor/robot" version="0.2.1" lang="en" />));
  const select = container.querySelector<HTMLSelectElement>('select[id*="services-native-scene"]')!;
  expect([...select.options].map((option) => option.value)).toEqual(['', 'pick-place', 'navigation']);
  await act(async () => { select.value = 'navigation'; select.dispatchEvent(new Event('change', { bubbles: true })); });
  await act(async () => [...container.querySelectorAll('button')].find((button) => button.textContent === 'Save configuration')!.click());
  expect(mocks.put).toHaveBeenCalledWith('vendor/robot', '0.2.1', 'abc', { values: { services: { native: { scene: 'navigation' } } } });
  await act(async () => root.unmount());
});
