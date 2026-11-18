import React, { useEffect, useRef, useState } from 'react';
import type { NodeGraph, ProcessStatus } from '../../services/isos/node-graph';
import { decodeBinaryVisionFrame, getGraphVisionBindings, graphVisionSocketUrl, type VisionBinding } from '../../services/isos/vision';
import type { Language } from '../../types';
import './eos-workbench.scss';

export interface VisionMonitor { path: string; label: string }
export function graphVisionMonitors(graph: NodeGraph | null): VisionMonitor[] {
  if (!graph) return [];
  const result: VisionMonitor[] = [];
  for (const [coreId, core] of Object.entries(graph.cores)) for (const [layerId, layer] of Object.entries(core.layers)) for (const [groupId, group] of Object.entries(layer.groups)) for (const [nodeId, node] of Object.entries(group.nodes)) {
    if (node.node_type === 'isos.ui.vision.monitor@v0.2') result.push({ path: `${coreId}/${layerId}/${groupId}/${nodeId}`, label: node.label || nodeId });
  }
  return result;
}

type Phase = 'stopped' | 'stale' | 'connecting' | 'waiting' | 'receiving' | 'unavailable' | 'disconnected';
function VisionNodeView({ lang, monitor, revision, status, visible }: { lang: Language; monitor: VisionMonitor; revision: string; status: ProcessStatus | null; visible: boolean }): React.ReactElement {
  const copy = lang === 'cn' ? { stopped: '已停止', stale: '运行图已过期', connecting: '正在连接', waiting: '等待画面', receiving: '正在接收画面', unavailable: '画面不可用', disconnected: '连接已断开', retry: '重试', received: '收到的画面', source: '相机来源', format: '格式', fps: '接收帧率', monitor: '监视节点', transfer: '传输节点', interval: '传输间隔', unbounded: '未设置', frame: '帧 ID', sequence: '捕获输出序号', sourceSequence: '源采样序号', clock: '源时钟', gap: '跳过的捕获输出', sampleInterval: '源采样间隔' } : { stopped: 'Stopped', stale: 'Running graph is stale', connecting: 'Connecting', waiting: 'Waiting for frame', receiving: 'Receiving frames', unavailable: 'Frame unavailable', disconnected: 'Disconnected', retry: 'Retry', received: 'Received frame', source: 'Camera source', format: 'Format', fps: 'Delivered FPS', monitor: 'Monitor node', transfer: 'Transfer node', interval: 'Transfer interval', unbounded: 'Unset', frame: 'Frame ID', sequence: 'Capture output sequence', sourceSequence: 'Source sample sequence', clock: 'Source clock', gap: 'Skipped capture outputs', sampleInterval: 'Source sample interval' };
  const [phase, setPhase] = useState<Phase>('stopped');
  const [error, setError] = useState('');
  const [image, setImage] = useState('');
  const [details, setDetails] = useState<{ width: number; height: number; source: string; encoding: string; fps: number; frameId: string; outputSequence: number; sourceSequence: number; clock: string; skipped: number; sampleIntervalMs: number | null } | null>(null);
  const [binding, setBinding] = useState<VisionBinding | null>(null);
  const [attempt, setAttempt] = useState(0);
  const retryCount = useRef(0);
  const imageRef = useRef('');
  const frameTimes = useRef<number[]>([]);
  const current = visible && status?.state === 'running' && status.run_id_state === 'known' && Boolean(status.run_id) && Boolean(revision) && revision === status.saved_revision && revision === status.active_revision;
  const fence = current ? `${status?.run_id}:${revision}:${monitor.path}` : '';
  useEffect(() => {
    let disposed = false;
    let socket: WebSocket | null = null;
    let retryTimer: number | null = null;
    let lastSequence = -1;
    let lastSourceTimeNs: number | null = null;
    let lastSourceClock = '';
    let skipped = 0;
    const clearImage = (): void => { if (imageRef.current) URL.revokeObjectURL(imageRef.current); imageRef.current = ''; frameTimes.current = []; setImage(''); setDetails(null); };
    const retry = (): void => {
      if (disposed || retryTimer !== null) return;
      const delay = Math.min(8000, 500 * 2 ** Math.min(retryCount.current++, 4));
      retryTimer = window.setTimeout(() => { retryTimer = null; if (!disposed && document.visibilityState === 'visible') setAttempt((value) => value + 1); }, delay);
    };
    const onVisible = (): void => { if (document.visibilityState === 'visible' && !socket && retryTimer === null) setAttempt((value) => value + 1); };
    document.addEventListener('visibilitychange', onVisible);
    clearImage(); setBinding(null); setError('');
    if (!current || !status?.run_id) { retryCount.current = 0; setPhase(!visible || status?.state === 'stopped' ? 'stopped' : 'stale'); return () => { disposed = true; document.removeEventListener('visibilitychange', onVisible); }; }
    const runId = status.run_id;
    setPhase('connecting');
    void (async () => {
      try {
        const catalog = await getGraphVisionBindings();
        if (disposed) return;
        const matches = catalog.bindings.filter((item) => item.monitor_path === monitor.path);
        if (catalog.run_id !== runId || catalog.revision !== revision || matches.length !== 1) throw new Error('graph_binding_unavailable');
        const binding: VisionBinding = matches[0];
        setBinding(binding);
        const ws = new WebSocket(graphVisionSocketUrl()); ws.binaryType = 'arraybuffer'; socket = ws;
        ws.onopen = () => { if (!disposed) { retryCount.current = 0; ws.send(JSON.stringify({ monitor_path: monitor.path, run_id: runId, revision })); } };
        ws.onmessage = (event) => {
          if (disposed || socket !== ws) return;
          try {
            if (!(event.data instanceof ArrayBuffer)) throw new Error('frame_invalid');
            const { value, blob } = decodeBinaryVisionFrame(event.data, binding, runId, revision);
            const sequence = value.output_sequence as number;
            if (sequence <= lastSequence) return;
            if (lastSequence >= 0) skipped += Math.max(0, sequence - lastSequence - 1);
            const sourceTimeNs = value.sample.source_time_ns as number;
            const sourceClock = value.sample.clock as string;
            const sourceDeltaNs = lastSourceClock === sourceClock && lastSourceTimeNs !== null && Number.isSafeInteger(sourceTimeNs) && sourceTimeNs > lastSourceTimeNs ? sourceTimeNs - lastSourceTimeNs : null;
            const sampleIntervalMs = sourceDeltaNs === null ? null : sourceDeltaNs / 1_000_000;
            lastSequence = sequence;
            lastSourceTimeNs = Number.isSafeInteger(sourceTimeNs) ? sourceTimeNs : null;
            lastSourceClock = sourceClock;
            const now = performance.now();
            frameTimes.current = [...frameTimes.current.filter((time) => now - time < 2000), now];
            const times = frameTimes.current;
            const fps = times.length > 1 && now > times[0] ? Math.round((times.length - 1) * 10000 / (now - times[0])) / 10 : 0;
            const url = URL.createObjectURL(blob);
            if (imageRef.current) URL.revokeObjectURL(imageRef.current);
            imageRef.current = url; setImage(url); setPhase('receiving'); setError('');
            setDetails({ width: value.width_px, height: value.height_px, source: binding.source_path, encoding: value.encoding.toUpperCase(), fps, frameId: value.sample.frame_id, outputSequence: sequence, sourceSequence: value.sample.sequence, clock: sourceClock, skipped, sampleIntervalMs });
          } catch (caught) { if (!disposed) { clearImage(); setPhase('unavailable'); setError(caught instanceof Error ? caught.message : 'frame_invalid'); } }
        };
        ws.onerror = () => { if (!disposed) setError('socket_error'); };
        ws.onclose = () => { if (!disposed) { socket = null; clearImage(); setPhase('disconnected'); retry(); } };
        setPhase('waiting');
      } catch (caught) { if (!disposed) { setPhase('unavailable'); setError(caught instanceof Error ? caught.message : 'catalog_unavailable'); retry(); } }
    })();
    return () => { disposed = true; if (retryTimer !== null) window.clearTimeout(retryTimer); document.removeEventListener('visibilitychange', onVisible); socket?.close(); clearImage(); };
  }, [fence, attempt, visible]);
  return <section className="eos-vision__node" aria-label={monitor.label}><h2>{monitor.label}</h2><p role="status">{copy[phase]}</p>{error && <p role="alert">{error}</p>}{phase === 'disconnected' && <button type="button" onClick={() => setAttempt((value) => value + 1)}>{copy.retry}</button>}{binding && <dl className="eos-vision__details"><dt>{copy.monitor}</dt><dd><code>{binding.monitor_path}</code></dd><dt>{copy.transfer}</dt><dd><code>{binding.transfer_path}</code></dd><dt>{copy.source}</dt><dd><code>{binding.source_path}</code></dd><dt>{copy.interval}</dt><dd>{binding.interval_ms === null ? copy.unbounded : `${binding.interval_ms} ms`}</dd></dl>}<div className="eos-vision__node-content"><div className="eos-vision__image">{image && phase === 'receiving' ? <img src={image} alt={monitor.label} /> : <p>{copy[phase]}</p>}</div>{details && phase === 'receiving' && <dl className="eos-vision__details"><dt>{copy.received}</dt><dd>{details.width} × {details.height} px</dd><dt>{copy.format}</dt><dd>{details.encoding}</dd><dt>{copy.fps}</dt><dd>{details.fps.toFixed(1)} Hz</dd><dt>{copy.frame}</dt><dd><code>{details.frameId}</code></dd><dt>{copy.sequence}</dt><dd>{details.outputSequence}</dd><dt>{copy.sourceSequence}</dt><dd>{details.sourceSequence}</dd><dt>{copy.clock}</dt><dd>{details.clock}</dd><dt>{copy.gap}</dt><dd>{details.skipped}</dd><dt>{copy.sampleInterval}</dt><dd>{details.sampleIntervalMs === null ? '—' : `${details.sampleIntervalMs.toFixed(1)} ms`}</dd></dl>}</div></section>;
}

export function GraphVision({ lang, graph, revision, status, visible }: { lang: Language; graph: NodeGraph | null; revision: string; status: ProcessStatus | null; visible: boolean }): React.ReactElement | null {
  const monitors = graphVisionMonitors(graph);
  if (!monitors.length) return null;
  return <main className="isos-vision-page" aria-label={lang === 'cn' ? '视觉' : 'Vision'}><h1>{lang === 'cn' ? '视觉' : 'Vision'}</h1><div className="isos-vision-page__grid">{monitors.map((monitor) => <VisionNodeView key={monitor.path} lang={lang} monitor={monitor} revision={revision} status={status} visible={visible} />)}</div></main>;
}
