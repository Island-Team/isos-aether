import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it, vi } from 'vitest';
vi.mock('../../services/isos/api-client', () => ({ request: vi.fn() }));
import { EiWorkbench, eiTabs } from './EiWorkbench';

describe('EI workbench navigation', () => {
  it('shows overview, logs and graph navigation in both languages', () => {
    expect(eiTabs('en')).toEqual({ overview: 'Overview', logs: 'Logs' });
    expect(eiTabs('cn')).toEqual({ overview: '概览', logs: '日志' });
    const html = renderToStaticMarkup(React.createElement(EiWorkbench, { lang: 'en', onOpenGraph: () => undefined }));
    expect(html).toContain('Overview');
    expect(html).toContain('Logs');
    expect(html).toContain('Graph');
    expect(html).toContain('No EI process status is available');
  });
});
