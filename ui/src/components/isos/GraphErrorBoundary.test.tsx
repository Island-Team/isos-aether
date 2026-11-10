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
  expect(getBrowserLogs().some((line) => line.message.includes('invalid setting descriptor'))).toBe(true);
  await act(async () => (container.querySelector('button') as HTMLButtonElement).click());
  expect(openLogs).toHaveBeenCalledOnce();
  await act(async () => root.unmount());
});
