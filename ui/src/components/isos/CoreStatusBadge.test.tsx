// @vitest-environment jsdom
import React, { act } from 'react';
import { createRoot } from 'react-dom/client';
import { expect, it } from 'vitest';
import { CoreStatusBadge } from './NodeGraphManager';

it('opens EI and EOS readiness details only when their status is clicked', async () => {
  (globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
  const host = document.createElement('div');
  document.body.append(host);
  const root = createRoot(host);
  await act(async () => root.render(<CoreStatusBadge name="EOS" lang="en" core={{ core: 'eos', path: '/bin/eos', version: '0.2.1', compatible: true, configured: true, graph_configured: true, running: false, ready: false, reason: 'pending_apply', action: 'Apply' }} />));
  const details = host.querySelector('details')!;
  expect(details.open).toBe(false);
  expect(host.querySelector('summary')?.getAttribute('aria-label')).toBe('EOS: Attention');
  await act(async () => (host.querySelector('summary') as HTMLElement).click());
  expect(details.open).toBe(true);
  expect(details.querySelector('[role="note"]')?.textContent).toContain('Graph file saved; Apply to update deployment configuration');
  await act(async () => root.unmount());
  host.remove();
});
