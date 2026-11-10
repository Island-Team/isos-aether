import React from 'react';
import { browserLog } from '@/services/isos/browser-logs';
import type { Language } from '@/types';

export class GraphErrorBoundary extends React.Component<{ children: React.ReactNode; lang: Language; onOpenLogs: () => void }, { error: Error | null }> {
  state = { error: null as Error | null };

  static getDerivedStateFromError(error: Error): { error: Error } { return { error }; }

  componentDidCatch(error: Error, info: React.ErrorInfo): void {
    browserLog('error', `Graph editor render failed: ${error.stack || error.message}\n${info.componentStack}`);
  }

  render(): React.ReactNode {
    if (!this.state.error) return this.props.children;
    return <div role="alert" className="isos-workbench__graph-error">
      <h2>{this.props.lang === 'cn' ? '节点图显示失败' : 'Node graph display failed'}</h2>
      <p>{this.props.lang === 'cn' ? '请查看浏览器日志，随后重新加载页面。' : 'Open browser logs, then reload the page.'}</p>
      <button type="button" onClick={this.props.onOpenLogs}>{this.props.lang === 'cn' ? '打开日志' : 'Open logs'}</button>
    </div>;
  }
}
