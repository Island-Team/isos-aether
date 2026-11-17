// @vitest-environment jsdom
import React from 'react';
import { act } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, expect, it, vi } from 'vitest';
import { GraphErrorBoundary } from './GraphErrorBoundary';
import { getBrowserLogs } from '@/services/isos/browser-logs';

afterEach(() => { document.body.innerHTML = ''; vi.restoreAllMocks(); });

it('keeps the workbench shell available and records a graph render failure', async () => {
  vi.stubGlobal('IS_REACT_ACT_ENVIRONMENT', true);
  vi.spyOn(console, 'error').mockImplementation(() => undefined);
  const container = document.createElement('div'); document.body.appendChild(container);
  const root = createRoot(container);
  const Broken = (): React.ReactElement => { throw new Error('invalid setting descriptor'); };
  const openLogs = vi.fn();
  await act(async () => root.render(<GraphErrorBoundary lang="en" onOpenLogs={openLogs}><Broken /></GraphErrorBoundary>));
  expect(container.textContent).toContain('Node graph display failed');
  expect(container.querySelector('pre')?.textContent).toBe('invalid setting descriptor');
  expect(getBrowserLogs().some((line) => line.message.includes('invalid setting descriptor'))).toBe(true);
  await act(async () => (container.querySelector('button') as HTMLButtonElement).click());
  expect(openLogs).toHaveBeenCalledOnce();
  await act(async () => root.unmount());
});

it('retries a transient graph render failure without reloading the workbench', async () => {
  vi.stubGlobal('IS_REACT_ACT_ENVIRONMENT', true);
  vi.spyOn(console, 'error').mockImplementation(() => undefined);
  const container = document.createElement('div'); document.body.appendChild(container);
  const root = createRoot(container);
  let fail = true;
  const SometimesBroken = (): React.ReactElement => {
    if (fail) throw new Error('temporary catalog error');
    return <p>Graph restored</p>;
  };
  await act(async () => root.render(<GraphErrorBoundary lang="en" onOpenLogs={() => undefined}><SometimesBroken /></GraphErrorBoundary>));
  fail = false;
  await act(async () => [...container.querySelectorAll('button')].find((button) => button.textContent === 'Retry display')?.click());
  expect(container.textContent).toContain('Graph restored');
  await act(async () => root.unmount());
});
