import React, { useCallback, useEffect, useRef, useState } from 'react';
import { getNodeGraph, getProcessStatus, type ProcessStatus } from '@/services/isos/node-graph';
import { DEFAULT_MONITOR_REFRESH_HZ, monitorRefreshHz } from '@/services/isos/arm-monitor';
import type { Language } from '@/types';
import { publishUiMessage } from '@/services/isos/ui-messages';
import { WorkspacePanel } from './WorkspacePanel';
import { ArmMonitorView, useArmMonitor } from './ArmMonitor';
import type { WorkspaceLayout, Surface, PanelLayout } from './workspace-layout';
import './eos-workbench.scss';

interface Props { lang: Language; onOpenGraph: () => void; onOpenLogs?: () => void; graphDirty: boolean; layout?: WorkspaceLayout; viewport?: { width: number; height: number }; onPanelChange?: (name: Surface, patch: Partial<PanelLayout>) => void; onRobot?: (id: string) => void; onStatus?: (status: ProcessStatus | null) => void; onConnectionError?: (message: string) => void; isRunStopFenced?: (runId: string | null) => boolean; onRunStopIntent?: (runId: string | null) => void; onStopOutcome?: (feedback: { text: string; alert: boolean }, terminalRun?: string | null) => void }

/** The setup surface reports owner process health; graph ports depend on registered profiles. */
export function EosWorkbench({ lang, onOpenGraph, onOpenLogs, layout, viewport, onPanelChange, onStatus, onConnectionError }: Props): React.ReactElement {
  const [status, setStatus] = useState<ProcessStatus | null>(null);
  const [monitorRateHz, setMonitorRateHz] = useState(DEFAULT_MONITOR_REFRESH_HZ);
  const monitorReading = useArmMonitor(Boolean(layout?.monitor.open && !layout.monitor.collapsed), monitorRateHz);
  useEffect(() => {
    let active = true;
    if (!status?.saved_revision) { setMonitorRateHz(DEFAULT_MONITOR_REFRESH_HZ); return; }
    void getNodeGraph().then((document) => { if (active) setMonitorRateHz(document.revision === status.saved_revision ? monitorRefreshHz(document.graph) : DEFAULT_MONITOR_REFRESH_HZ); }).catch(() => { if (active) setMonitorRateHz(DEFAULT_MONITOR_REFRESH_HZ); });
    return () => { active = false; };
  }, [status?.saved_revision]);
  const callbacks = useRef({ onStatus, onConnectionError });
  callbacks.current = { onStatus, onConnectionError };
  const copy = lang === 'cn'
    ? { title: 'EOS 进程健康', refresh: '刷新', graph: '节点图', logs: '日志', state: '部署状态', active: '运行中修订', empty: 'EOS 进程尚未启动。', unavailable: '无法读取进程健康状态。' }
    : { title: 'EOS process health', refresh: 'Refresh', graph: 'Graph', logs: 'Logs', state: 'Deployment state', active: 'Active revision', empty: 'EOS process is not running.', unavailable: 'Process health is unavailable.' };
  const refresh = useCallback(async (): Promise<void> => {
    try {
      const next = await getProcessStatus();
      setStatus(next);
      callbacks.current.onStatus?.(next);
      callbacks.current.onConnectionError?.('');
    } catch (error) {
      setStatus(null);
      callbacks.current.onStatus?.(null);
      const message = error instanceof Error ? error.message : copy.unavailable;
      callbacks.current.onConnectionError?.(message);
      publishUiMessage(message, 'error');
    }
  }, [copy.unavailable]);
  useEffect(() => {
    void refresh();
    const timer = window.setInterval(() => { if (document.visibilityState === 'visible') void refresh(); }, 5000);
    return () => window.clearInterval(timer);
  }, [refresh]);
  const eos = status?.processes.filter((process) => process.id === 'eos' || process.id.startsWith('eos/')) || [];
  const content = <main className="eos-control"><header><h1>{copy.title}</h1><div><button type="button" onClick={() => void refresh()}>{copy.refresh}</button><button type="button" onClick={onOpenGraph}>{copy.graph}</button>{onOpenLogs && <button type="button" onClick={onOpenLogs}>{copy.logs}</button>}</div></header><dl><dt>{copy.state}</dt><dd>{status?.state || '—'}</dd><dt>{copy.active}</dt><dd><code>{status?.active_revision || '—'}</code></dd></dl>{eos.length ? <ul>{eos.map((process) => <li key={process.id}><strong>{process.id}</strong> · {process.state || (process.pid ? 'running' : 'stopped')} · PID {process.pid ?? '—'}{process.last_error && <p role="alert">{process.last_error}</p>}</li>)}</ul> : <p>{copy.empty}</p>}</main>;
  return layout && viewport && onPanelChange
    ? <><WorkspacePanel title={copy.title} lang={lang} layout={layout.control} viewport={viewport} onChange={(patch) => onPanelChange('control', patch)}>{content}</WorkspacePanel><WorkspacePanel title={lang === 'cn' ? '机械臂监控' : 'Arm monitor'} lang={lang} layout={layout.monitor} viewport={viewport} onChange={(patch) => onPanelChange('monitor', patch)}><ArmMonitorView lang={lang} reading={monitorReading} expectedRunId={!status ? undefined : status.state === 'running' || status.state === 'degraded' ? status.run_id : null} expectedRevision={!status ? undefined : status.state === 'running' || status.state === 'degraded' ? status.active_revision : null} /></WorkspacePanel></>
    : content;
}
