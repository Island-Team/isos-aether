import React, { useEffect, useState } from 'react';
import { DEFAULT_MONITOR_REFRESH_HZ, getArmMonitor, type ArmMonitorSnapshot } from '@/services/isos/arm-monitor';
import { IsosApiError } from '@/services/isos/transport';
import type { Language } from '@/types';
import './arm-monitor.scss';

export interface ArmMonitorReading { snapshot: ArmMonitorSnapshot | null; receivedAt: number | null; error: string; errorCode?: string }

/** One request at a time, at most five per second while the surface is visible. */
export function useArmMonitor(enabled: boolean, refreshHz = DEFAULT_MONITOR_REFRESH_HZ): ArmMonitorReading {
  const [reading, setReading] = useState<ArmMonitorReading>({ snapshot: null, receivedAt: null, error: '' });
  useEffect(() => {
    if (!enabled) { setReading({ snapshot: null, receivedAt: null, error: '' }); return; }
    let active = true;
    let busy = false;
    let lastStartedAt = 0;
    let controller: AbortController | null = null;
    const rate = Number.isFinite(refreshHz) ? Math.max(1, Math.min(5, refreshHz)) : DEFAULT_MONITOR_REFRESH_HZ;
    const intervalMs = Math.ceil(1000 / rate);
    const refresh = async (): Promise<void> => {
      if (!active || busy || document.visibilityState !== 'visible') return;
      const now = Date.now();
      if (now - lastStartedAt < intervalMs) return;
      lastStartedAt = now;
      busy = true; controller = new AbortController();
      try {
        const snapshot = await getArmMonitor(controller.signal);
        if (active) setReading({ snapshot, receivedAt: snapshot.state === 'running' ? Date.now() : null, error: '' });
      } catch (error) {
        if (active) setReading({ snapshot: null, receivedAt: null, error: error instanceof Error ? error.message : String(error), errorCode: error instanceof IsosApiError ? error.code : undefined });
      } finally { busy = false; controller = null; }
    };
    void refresh();
    const timer = window.setInterval(() => void refresh(), intervalMs);
    document.addEventListener('visibilitychange', refresh);
    return () => { active = false; controller?.abort(); window.clearInterval(timer); document.removeEventListener('visibilitychange', refresh); };
  }, [enabled, refreshHz]);
  return reading;
}

export function ArmMonitorView({ lang, reading, expectedRunId, expectedRevision, compact = false }: { lang: Language; reading: ArmMonitorReading; expectedRunId?: string | null; expectedRevision?: string | null; compact?: boolean }): React.ReactElement {
  const copy = lang === 'cn'
    ? { title: '模拟机械臂状态', stopped: '已停止', unavailable: '监测状态不可用', waiting: '等待本次运行的机械臂状态', ready: '就绪', degraded: '降级', stale: '过期', joints: '关节位置', sampled: '采样序号', age: '收到时间', reason: '原因' }
    : { title: 'Mock arm state', stopped: 'Stopped', unavailable: 'Monitor unavailable', waiting: 'Waiting for this run’s arm state', ready: 'Ready', degraded: 'Degraded', stale: 'Stale', joints: 'Joint positions', sampled: 'Sample sequence', age: 'Received', reason: 'Reason' };
  const snapshot = reading.snapshot;
  const errorText = reading.errorCode === 'graph_fence_stale'
    ? (lang === 'cn' ? '当前运行使用旧版节点图。停止或恢复该运行后，再运行已保存的节点图。' : 'The active run uses an older graph. Stop or recover it, then run the saved graph.')
    : reading.error;
  const observation = snapshot?.state === 'running' && (expectedRunId === undefined || snapshot.run_id === expectedRunId) && (expectedRevision === undefined || snapshot.active_revision === expectedRevision) ? snapshot.observation : null;
  const state = reading.error ? copy.unavailable : snapshot?.state === 'stopped' || expectedRunId === null ? copy.stopped : observation ? copy[observation.state] : copy.waiting;
  return <section className={`arm-monitor${compact ? ' arm-monitor--compact' : ''}`} aria-label={copy.title}>
    {!compact && <h2>{copy.title}</h2>}
    <p role={reading.error || observation?.state === 'degraded' || observation?.state === 'stale' ? 'alert' : 'status'}><strong>{state}</strong>{errorText && ` · ${errorText}`}{observation?.reason_code && ` · ${copy.reason}: ${observation.reason_code}`}</p>
    {observation && <>{!compact && <p>{copy.sampled}: {observation.sequence} · {copy.age}: {reading.receivedAt ? new Date(reading.receivedAt).toLocaleTimeString(lang === 'cn' ? 'zh-CN' : 'en-US') : '—'}</p>}{observation.joint_positions_rad && <div><strong>{copy.joints} (rad)</strong><ol>{observation.joint_positions_rad.map((position, index) => <li key={index}><span>{observation.joint_names[index]}</span><output>{position.toFixed(3)}</output></li>)}</ol></div>}</>}
  </section>;
}
