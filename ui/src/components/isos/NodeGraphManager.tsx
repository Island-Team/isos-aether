import React, { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import ReactFlow, { Background, Handle, MiniMap, Position, ReactFlowProvider, SelectionMode, useUpdateNodeInternals, type Connection, type Edge, type Node, type NodeChange, type NodeProps, type ReactFlowInstance } from 'reactflow';
import 'reactflow/dist/style.css';
import { catalogTypeKey, combineNodeTypeCatalogs, graphPackagePins, type NodeOptions, getGraphCatalogSnapshot, getNewNodeGraph, getNodeTypes, getGraphPool, getGraphPoolEntry, putGraphPoolEntry, getNodeGraph, getInstalledPackages, getProcessStatus, getCoreReadiness, getNodeOptions, graphCatalogReadOnlyReason, nodeGraphHasEditableShape, nodeGraphReadOnlyReason, validateConnection, validateNodeGraph, type CoreReadiness, type CoreReadinessEntry, type DataTypeDefinition, type GraphPoolEntry, type InstalledPackage, type NodeGraph, type NodeGraphDiagnostic, type NodeInstance, type NodeTypeCatalog, type NodeTypeDefinition, type ProcessStatus, type ValidatedEdge } from '@/services/isos/node-graph';
import { subscribeRuntimeStatus } from '@/services/isos/runtime-status-stream';
import type { Language } from '@/types';
import { hasActiveDeployment } from './deployment-phase';
import { addNode, addableTypes, admittedConnection, bindPrimitiveTarget, canonicalNodePath, connectNodes, disconnectInput, graphNodes, inputSources, nextNodeId, nodePath, packageGraphDraft, portInput, removeNode, renameNode, sameNodePath, updateDesiredRate, updateNode, type GraphNode } from './node-graph-edits';
import { interfaceVisual, nodeVisualStyle, nodeInteractionRoles, portVisual, dataTypeColor } from './node-visuals';
import { applyNodeDimensions, applyNodeSelection, asyncDemoLayout, DEFAULT_GRAPH_PANEL_WIDTHS, fitGraphPanelWidths, graphLayoutKey, graphMirrorKey, graphPanelWidthsKey, parseGraphPanelWidths, parseNodeMirrors, parseNodePositions, preserveNodePositions, resizeGraphPanel, removeSelectedNodes, type GraphPanelWidths, type NodeMirrors, type NodePositions, type NodeSizes } from './graph-canvas-state';
import { edgeContracts, edgeInputDiagnostics } from './edge-idl';
import { validatedApply } from './validated-apply';
import { clampInfoCard } from './node-info-card';
import { processRunFeedback } from './process-run-feedback';
import { startupPhaseText } from './startup-phase';
import { stopReplyFeedback, stopFailureFeedback } from './stop-feedback';
import { requestRunStop } from './run-stop-fence';
import { deploymentStopEpoch, runDeployment, stopDeploymentConverged } from './deployment-control';
import { browserLog } from '@/services/isos/browser-logs';
import { publishUiMessage } from '@/services/isos/ui-messages';
import { callExternalNode } from '@/services/isos/external-node';
import { getButtonActivationResult, newButtonEventId, sendButtonActivation, type ButtonActivationResult } from '@/services/isos/node-events';
import { IsosApiError } from '@/services/isos/api-client';
import { draftValidationStamp, validationDisplayIsCurrent, validationResponseIsCurrent, type DraftValidationStamp } from './draft-validation';
import { CatalogSelector, settingOptionGroup, unavailableSelections } from './CatalogSelector';
import { CatalogBrowser, packageAccent, packagePin } from './CatalogBrowser';
import { PackageLibrary, installedPin } from './PackageLibrary';
import { PackageConfigPanel } from './PackageConfigPanel';
import { GraphDependencyRepair } from './GraphDependencyRepair';
import { ArmMonitorView, useArmMonitor, type ArmMonitorReading } from './ArmMonitor';
import { DEFAULT_MONITOR_REFRESH_HZ, monitorRefreshHz } from '@/services/isos/arm-monitor';
import { catalogErrorIsStable, catalogReloadGraph, catalogReloadIsCurrent, catalogSelectionBlocks, displayNodeOptions, draftAdapterOptions, optionResponseIsCurrent, type OptionSnapshot } from './catalog-options';
import './node-graph-manager.scss';

interface Props { lang: Language; visible?: boolean; panelMenu?: React.ReactNode; onDirtyChange?: (dirty: boolean) => void; onNoticesChange?: (notices: { text: string; alert: boolean }[]) => void; onOpenPanel?: (name: 'logs') => void; onOpenPage?: (name: 'ei' | 'eos') => void; onStopIntent?: (runId: string | null) => void; onStopOutcome?: (feedback: { text: string; alert: boolean }, terminalRun?: string | null) => void }
type InspectorTarget = 'none' | 'package' | 'type' | 'node' | 'edge' | 'multi';
interface FlowData { entry: GraphNode; error: boolean; mirrored: boolean; lane?: 'forward' | 'return'; lang: Language; dataTypes: DataTypeDefinition[]; currentRun?: boolean; runId?: string | null; activeRevision?: string | null; monitorReading?: ArmMonitorReading; onSelect?: (path: string) => void }
const COPY = {
  en: { title: 'Node graph', connect: 'Connect to local IsOS', add: 'Add node', refresh: 'Refresh', validate: 'Check graph', apply: 'Apply graph', saved: 'Graph file applied and core configuration generated. Restart to use this revision.', valid: 'Graph is valid.', dirty: 'Unsaved changes', clean: 'Saved', catalog: 'Node library', inspector: 'Node configuration', diagnostics: 'Diagnostics', noSelection: 'Select a node to edit it.', selectedCount: 'nodes selected', noNodes: 'Add a registered node type to start.', id: 'Node ID', label: 'Label', description: 'Description', settings: 'Settings', gui: 'Show package service window when available', details: 'Hardware and advanced fields (JSON)', applyDetails: 'Apply fields', remove: 'Remove node', capabilities: 'Capabilities', source: 'Source', target: 'Target', invalid: 'Invalid JSON object.', duplicate: 'Node ID already exists or is invalid.', port: 'Connect matching typed ports by dragging from an output to an input.', robot: 'Robot', name: 'Name', lifecycle: 'Lifecycle', type: 'Type', disconnected: 'IsOS node graph API is unavailable.', stale: 'Graph changed on the server. Refresh and retry.', discard: 'Discard unsaved changes?', choose: 'Choose a type', process: 'Process status', noStatus: 'Process status is not available from this IsOS build.', search: 'Search types', undo: 'Undo', redo: 'Redo', disconnect: 'Disconnect line', hint: 'Left drag selects nodes; middle drag pans. Right-click to add or disconnect. Delete removes selected nodes.', savedRevision: 'Saved', activeRevision: 'Active' },
  cn: { title: '节点图', connect: '连接本机 IsOS', add: '添加节点', refresh: '刷新', validate: '检查节点图', apply: '应用节点图', saved: '图库文件已应用，核心配置已生成。重启后使用此修订。', valid: '节点图有效。', dirty: '未保存更改', clean: '已保存', catalog: '节点库', inspector: '节点配置', diagnostics: '诊断', noSelection: '选择节点后编辑配置。', selectedCount: '个节点已选中', noNodes: '添加已注册的节点类型以开始。', id: '节点 ID', label: '标签', description: '描述', settings: '设置', gui: '可用时显示软件包服务窗口', details: '硬件与高级字段（JSON）', applyDetails: '应用字段', remove: '删除节点', capabilities: '能力', source: '来源', target: '目标', invalid: 'JSON 对象无效。', duplicate: '节点 ID 已存在或无效。', port: '从输出端口拖到类型相同的输入端口以连接。', robot: '机器人', name: '名称', lifecycle: '生命周期', type: '类型', disconnected: 'IsOS 节点图 API 不可用。', stale: '服务器上的图已更改。刷新后重试。', discard: '放弃未保存的更改？', choose: '选择类型', process: '进程状态', noStatus: '当前 IsOS 版本未提供进程状态。', search: '搜索类型', undo: '撤销', redo: '重做', disconnect: '断开连线', hint: '左键拖动框选节点，中键拖动平移。右键添加节点或断开连线。Delete 删除选中节点。', savedRevision: '已保存', activeRevision: '运行中' },
} as const;

export function CoreStatusBadge({ name, core, lang }: { name: 'EI' | 'EOS'; core?: CoreReadinessEntry; lang: Language }): React.ReactElement {
  const state = !core ? 'unknown' : core.enabled === false ? 'disabled' : core.ready ? 'ready' : 'attention';
  const labels = lang === 'cn'
    ? { ready: '就绪', attention: '需处理', disabled: '未启用', unknown: '未知', running: '运行中', stopped: '已停止', path: '程序路径', version: '版本', reason: '原因', action: '建议' }
    : { ready: 'Ready', attention: 'Attention', disabled: 'Disabled', unknown: 'Unknown', running: 'Running', stopped: 'Stopped', path: 'Binary path', version: 'Version', reason: 'Reason', action: 'Action' };
  return <details className={`node-graph-core-status node-graph-core-status--${state}`}>
    <summary role="button" aria-label={`${name}: ${labels[state]}`}><span className="node-graph-core-status__dot" aria-hidden="true" />{name} · {labels[state]}</summary>
    <div className="node-graph-core-status__detail" role="note"><strong>{name}</strong><p>{labels[state]} · {core ? (core.running ? labels.running : labels.stopped) : labels.unknown}</p><p>{labels.version}: {core?.version || '—'}</p><p>{labels.path}: <code>{core?.path || '—'}</code></p>{core?.reason && <p>{labels.reason}: {core.reason === 'pending_apply' ? (lang === 'cn' ? '图库文件已保存，等待应用以更新部署配置' : 'Graph file saved; Apply to update deployment configuration') : core.reason}</p>}{core?.action && <p>{labels.action}: {core.action}</p>}</div>
  </details>;
}

export function FlowNode({ id, data, selected }: NodeProps<FlowData>): React.ReactElement {
  const { entry } = data;
  const updateNodeInternals = useUpdateNodeInternals();
  useLayoutEffect(() => { updateNodeInternals(id); }, [id, data.mirrored, updateNodeInternals]);
  const [open, setOpen] = useState(false);
  const [pendingEvent, setPendingEvent] = useState<string | null>(null);
  const [lastEventId, setLastEventId] = useState<string | null>(null);
  const [eventResult, setEventResult] = useState<ButtonActivationResult | null>(null);
  const [eventError, setEventError] = useState('');
  const [eventUncertain, setEventUncertain] = useState(false);
  const [externalPayload, setExternalPayload] = useState('{}');
  const [externalReply, setExternalReply] = useState('');
  const [externalPending, setExternalPending] = useState(false);
  const [submittedFence, setSubmittedFence] = useState<string | null>(null);
  const eventPollAbort = useRef<AbortController | null>(null);
  const [location, setLocation] = useState({ left: 8, top: 8 });
  const hideTimer = useRef<number | null>(null);
  const hoverTimer = useRef<number | null>(null);
  const pointerFocus = useRef(false);
  const cardRef = useRef<HTMLElement | null>(null);
  const currentFence = data.currentRun && data.runId && data.activeRevision ? `${data.runId}:${data.activeRevision}` : null;
  const currentFenceRef = useRef(currentFence);
  currentFenceRef.current = currentFence;
  const place = (x: number, y: number): void => setLocation(clampInfoCard(x + 16, y + 16, 340, 340, window.innerWidth, window.innerHeight));
  const cancelHover = (): void => { if (hoverTimer.current !== null) { window.clearTimeout(hoverTimer.current); hoverTimer.current = null; } };
  const show = (x: number, y: number): void => { cancelHover(); if (hideTimer.current !== null) window.clearTimeout(hideTimer.current); place(x, y); setOpen(true); };
  const showAfterPause = (x: number, y: number): void => { if (open || pointerFocus.current) return; cancelHover(); hoverTimer.current = window.setTimeout(() => show(x, y), 550); };
  const hideSoon = (): void => { cancelHover(); hideTimer.current = window.setTimeout(() => setOpen(false), 120); };
  useEffect(() => () => { cancelHover(); if (hideTimer.current !== null) window.clearTimeout(hideTimer.current); }, []);
  useEffect(() => () => eventPollAbort.current?.abort(), []);
  useEffect(() => { if (!open) return; const clamp = (): void => setLocation((value) => clampInfoCard(value.left, value.top, 340, 340, window.innerWidth, window.innerHeight)); window.addEventListener('resize', clamp); return () => window.removeEventListener('resize', clamp); }, [open]);
  const labels = data.lang === 'cn' ? { info: '节点信息', type: '类型描述', instance: '实例描述', input: '输入', output: '输出', missing: '目录无描述', close: '关闭' } : { info: 'Node information', type: 'Type description', instance: 'Instance description', input: 'Inputs', output: 'Outputs', missing: 'No catalog description', close: 'Close' };
  const buttonNode = entry.node.node_type === 'isos.ui.button.trigger@v0.2';
  const externalUiNode = entry.definition?.implementation?.interface === 'isos.ui.external.v1'
    && entry.definition.implementation.version === '1'
    && entry.definition.implementation.lifecycle_adapter === 'unix-socket-v1';
  const interactionRoles = nodeInteractionRoles(entry.definition);
  const roleOrder = ['publisher', 'subscriber', 'service', 'action'] as const;
  const roleLabels = data.lang === 'cn'
    ? { publisher: '发布', subscriber: '订阅', service: '服务', action: '动作' }
    : { publisher: 'Publisher', subscriber: 'Subscriber', service: 'Service', action: 'Action' };
  const callExternal = async (op: 'input' | 'read', port: string): Promise<void> => {
    if (!data.currentRun || !data.runId || !data.activeRevision || externalPending) return;
    const fence = `${data.runId}:${data.activeRevision}`;
    setExternalPending(true); setExternalReply('');
    try {
      const payload: unknown = op === 'read' ? null : JSON.parse(externalPayload);
      const reply = await callExternalNode({ schema_version: 1, node_path: entry.path, run_id: data.runId, active_revision: data.activeRevision, op, port, payload });
      setExternalReply(currentFenceRef.current === fence ? JSON.stringify(reply.payload, null, 2) : (data.lang === 'cn' ? '运行已改变；旧结果已丢弃。' : 'Run changed; prior result discarded.'));
    } catch (error) { setExternalReply(error instanceof Error ? error.message : String(error)); }
    finally { setExternalPending(false); }
  };
  const activate = async (): Promise<void> => {
    if (!buttonNode || !data.currentRun || !data.runId || !data.activeRevision || pendingEvent) return;
    const eventId = newButtonEventId();
    const fence = `${data.runId}:${data.activeRevision}`;
    const query = { schema_version: '0.2.0' as const, source_node: entry.path, run_id: data.runId, active_revision: data.activeRevision, event_id: eventId };
    const controller = new AbortController();
    eventPollAbort.current = controller;
    setSubmittedFence(fence); setPendingEvent(eventId); setLastEventId(eventId); setEventResult(null); setEventError(''); setEventUncertain(false);
    try {
      let result = await sendButtonActivation({ schema_version: '0.2.0', source_node: entry.path, output: 'activation', data_type: 'button_activation@v0.2', run_id: data.runId, active_revision: data.activeRevision, payload: { event_id: eventId, kind: 'activate' } });
      if (currentFenceRef.current === fence) {
        setEventResult(result);
        if (result.status === 'outcome_unknown') setEventUncertain(true);
      } else {
        setEventError(data.lang === 'cn' ? '运行已改变；旧结果仅用于排查。' : 'Run changed; the prior result is stale.');
      }
      for (let attempt = 0; attempt < 100 && ['accepted', 'running'].includes(result.status) && currentFenceRef.current === fence && !controller.signal.aborted; attempt += 1) {
        await new Promise<void>((resolve) => window.setTimeout(resolve, 300));
        if (controller.signal.aborted || currentFenceRef.current !== fence) break;
        try {
          result = await getButtonActivationResult(query, controller.signal);
          if (currentFenceRef.current === fence) setEventResult(result);
          if (result.status === 'outcome_unknown') setEventUncertain(true);
        } catch (error) {
          if (error instanceof IsosApiError && error.status < 500) throw error;
        }
      }
      if (['accepted', 'running'].includes(result.status) && !controller.signal.aborted && currentFenceRef.current === fence) {
        setEventUncertain(true);
        setEventError(data.lang === 'cn' ? '结果仍在处理中；请用同一事件 ID 查询。' : 'Result still pending; query with the same event ID.');
      }
    } catch (error) {
      if (controller.signal.aborted) return;
      const uncertain = !(error instanceof IsosApiError && error.status < 500 && error.code !== 'owner_outcome_unknown');
      if (uncertain && currentFenceRef.current === fence) {
        for (let attempt = 0; attempt < 100 && !controller.signal.aborted && currentFenceRef.current === fence; attempt += 1) {
          try {
            const result = await getButtonActivationResult(query, controller.signal);
            setEventResult(result);
            if (result.status === 'outcome_unknown') setEventUncertain(true);
            if (!['accepted', 'running'].includes(result.status)) return;
          } catch (resultError) {
            if (resultError instanceof IsosApiError && resultError.status === 409) break;
          }
          await new Promise<void>((resolve) => window.setTimeout(resolve, 300));
        }
      }
      setEventUncertain(uncertain);
      setEventError(error instanceof Error ? error.message : String(error));
    } finally { eventPollAbort.current = null; setPendingEvent(null); }
  };
  return <div className={`node-graph-card${interactionRoles.map((role) => ` node-graph-card--${role}`).join('')}${selected ? ' node-graph-card--selected' : ''}${data.error ? ' node-graph-card--error' : ''}${data.mirrored ? ' node-graph-card--mirrored' : ''}`} style={nodeVisualStyle(entry.definition) as React.CSSProperties} tabIndex={0} role="group" aria-label={`${entry.node.label || entry.node.id}: ${entry.node.node_type}. ${data.lang === 'cn' ? '按 Enter 编辑' : 'Press Enter to edit'}`} onMouseEnter={(event) => showAfterPause(event.clientX, event.clientY)} onMouseMove={(event) => showAfterPause(event.clientX, event.clientY)} onMouseLeave={hideSoon} onPointerDown={() => { pointerFocus.current = true; cancelHover(); setOpen(false); }} onPointerUp={() => { pointerFocus.current = false; }} onPointerCancel={() => { pointerFocus.current = false; }} onFocus={(event) => { if (pointerFocus.current || event.target !== event.currentTarget) return; const bounds = event.currentTarget.getBoundingClientRect(); show(bounds.right, bounds.top); }} onBlur={(event) => { if (!cardRef.current?.contains(event.relatedTarget as globalThis.Node)) hideSoon(); }} onKeyDown={(event) => { if (event.key === 'Escape') { setOpen(false); event.stopPropagation(); } else if (event.target === event.currentTarget && (event.key === 'Enter' || event.key === ' ')) { event.preventDefault(); event.stopPropagation(); setOpen(false); data.onSelect?.(entry.path); } }}>
    {interactionRoles.length > 0 && <div className="node-graph-card__role-strip" aria-label={interactionRoles.map((role) => roleLabels[role]).join(', ')}>{roleOrder.map((role) => <span key={role} className={`node-graph-card__role-slot node-graph-card__role-slot--${role}${interactionRoles.includes(role) ? ' node-graph-card__role-slot--active' : ''}`} title={roleLabels[role]}>{roleLabels[role].slice(0, 1)}</span>)}</div>}
    <div className="node-graph-card__heading"><small>{entry.path.split('/')[1]} / {entry.path.split('/')[2]}</small><span>{entry.node.node_type.split('@').pop() || 'v0.2'}</span></div>
    {data.lane && <small className="node-graph-card__lane">{data.lane === 'forward' ? (data.lang === 'cn' ? '前向 · 异步命令 →' : 'Forward · async command →') : (data.lang === 'cn' ? '← 返回 · 最新状态' : '← Return · latest state')}</small>}<strong>{entry.node.label || entry.node.id}</strong>
    {buttonNode && <div className="node-graph-card__actions nodrag nowheel"><small className="node-graph-card__command-route">{data.lang === 'cn' ? 'UI → EI · 激活事件' : 'UI → EI · activation event'}</small><button type="button" disabled={!data.currentRun || !!pendingEvent} onPointerDown={(event) => event.stopPropagation()} onClick={(event) => { event.stopPropagation(); void activate(); }}>{pendingEvent ? (data.lang === 'cn' ? '发送中…' : 'Sending…') : (data.lang === 'cn' ? '触发' : 'Activate')}</button></div>}
    {(/^(?:eos\.dispatcher|isos\.eos_dispatcher)\.primitive_/.test(entry.node.node_type)) && <div className="node-graph-card__command-route node-graph-card__command-route--eos">{data.lang === 'cn' ? 'UI → EOS · Dispatcher 控制端点' : 'UI → EOS · Dispatcher control endpoint'}</div>}
    {buttonNode && (eventResult || eventError) && <small className="node-graph-card__result" role={eventError || eventUncertain || submittedFence !== currentFence ? 'alert' : 'status'}>{lastEventId && <code>{lastEventId}</code>} · {submittedFence !== currentFence ? (data.lang === 'cn' ? '旧运行的结果；当前状态已改变' : 'Prior run result; current state changed') : eventError ? `${eventUncertain ? (data.lang === 'cn' ? '结果不确定；请勿自动重试' : 'Outcome uncertain; do not retry automatically') : ''} · ${eventError}` : eventResult ? `${eventResult.status}${eventResult.joint_positions_rad ? ` · ${eventResult.joint_positions_rad.join(', ')} rad` : ''}${eventUncertain ? (data.lang === 'cn' ? ' · 结果不确定；请勿自动重试' : ' · Outcome uncertain; do not retry automatically') : ''}` : ''}</small>}
    {entry.node.node_type === 'isos.ui.monitor.arm@v0.2' && <ArmMonitorView lang={data.lang} reading={data.monitorReading || { snapshot: null, receivedAt: null, error: '' }} expectedRunId={data.runId} expectedRevision={data.activeRevision} compact />}
    {externalUiNode && <div className="node-graph-card__external nodrag nowheel">
      {entry.definition?.inputs.map((port) => <div key={`input:${port.name}`}><label>{port.name} · {port.data_type}<textarea aria-label={`${port.name} JSON`} value={externalPayload} onChange={(event) => setExternalPayload(event.target.value)} rows={3} /></label><button type="button" disabled={!data.currentRun || externalPending} onClick={() => void callExternal('input', port.name)}>{data.lang === 'cn' ? '发送' : 'Send'}</button></div>)}
      {entry.definition?.outputs.map((port) => <button key={`read:${port.name}`} type="button" disabled={!data.currentRun || externalPending} onClick={() => void callExternal('read', port.name)}>{data.lang === 'cn' ? `读取 ${port.name}` : `Read ${port.name}`}</button>)}
      {externalReply && <pre role="status">{externalReply}</pre>}
    </div>}
    <div className="node-graph-card__badges"><span>{entry.definition?.core || 'unknown'}</span><span>{entry.definition?.kind || 'unknown'}</span>{interactionRoles.map((role) => <span key={role} className={`node-graph-card__role node-graph-card__role--${role}`}>{roleLabels[role]}</span>)}{entry.node.package && <span>{entry.node.package.id}@{entry.node.package.version}</span>}</div>
    <div className="node-graph-card__ports">{!entry.definition && <span className="node-graph-card__no-ports" role="alert">{data.lang === 'cn' ? '目录类型不可用；无法显示端口' : 'Catalog type unavailable; ports cannot be shown'}</span>}{entry.definition && !entry.definition.inputs.length && !entry.definition.outputs.length && <span className="node-graph-card__no-ports">{entry.node.node_type.endsWith('.health@v0.2') ? (data.lang === 'cn' ? '无图端口 · 归属方健康' : 'No graph ports · owner health') : (data.lang === 'cn' ? '无图端口' : 'No graph ports')}</span>}<div>{entry.definition?.inputs.map((port) => <div className={`node-graph-card__port ${portVisual(port).className}`} style={portVisual(port).style as React.CSSProperties} key={port.name}><Handle type="target" position={data.mirrored ? Position.Right : Position.Left} id={`in:${port.name}`} title={`${port.name}: ${port.data_type} · ${port.transport || 'in_process'} · ${port.delivery_mode || 'snapshot'}`} /><span>{port.name}{port.required ? ' *' : ''}</span><small>{port.data_type}</small></div>)}</div><div>{entry.definition?.outputs.map((port) => <div className={`node-graph-card__port node-graph-card__port--output ${portVisual(port).className}`} style={portVisual(port).style as React.CSSProperties} key={port.name}><span>{port.name}</span><small>{port.data_type}</small><Handle type="source" position={data.mirrored ? Position.Left : Position.Right} id={`out:${port.name}`} title={`${port.name}: ${port.data_type} · ${port.transport || 'in_process'} · ${port.delivery_mode || 'snapshot'}`} /></div>)}</div></div>
    {open && createPortal(<aside ref={cardRef} className="node-graph-info-card nodrag nowheel" role="note" aria-label={`${labels.info}: ${entry.node.label || entry.node.id}`} style={location} onMouseEnter={(event) => { event.stopPropagation(); if (hideTimer.current !== null) window.clearTimeout(hideTimer.current); }} onMouseLeave={(event) => { event.stopPropagation(); hideSoon(); }} onPointerDown={(event) => event.stopPropagation()} onKeyDown={(event) => { if (event.key === 'Escape') { setOpen(false); event.stopPropagation(); } }}><button type="button" onClick={() => setOpen(false)} aria-label={labels.close}>×</button><h3>{entry.node.label || entry.node.id}</h3><code>{entry.node.node_type}</code><p><strong>{labels.type}:</strong> {entry.definition?.description || labels.missing}</p><p><strong>{labels.instance}:</strong> {entry.node.description || '—'}</p>{([['input', entry.definition?.inputs], ['output', entry.definition?.outputs]] as const).map(([direction, ports]) => <section key={direction}><h4>{labels[direction]}</h4><ul>{ports?.map((port) => { const payload = data.dataTypes.find((item) => item.id === port.data_type); const schema = payload?.schema; const units = [...new Set(Object.values(schema?.properties || {}).map((field) => field['x-unit']).filter(Boolean))]; return <li key={port.name}><code>{port.name}</code> · {port.data_type}{units.length ? ` · ${units.join(', ')}` : ''}{port.description ? ` · ${port.description}` : ''}{port.transport ? ` · ${port.transport}` : ''}{port.delivery_mode ? ` · ${port.delivery_mode}` : ''}{port.contract ? ` · ${port.contract.owner}/${port.contract.id}@${port.contract.version}` : ' · static declaration'}{payload ? ` · ${payload.codec} · ${payload.media_type} · ${payload.max_payload_bytes} bytes · fields: ${Object.keys(schema?.properties || {}).join(', ') || '—'}` : ''}</li>; })}</ul></section>)}</aside>, document.body)}

  </div>;
}
const nodeTypes = { profile: FlowNode };
export function EdgeIdlPanel({ edge, entries, dataTypes = [], diagnostics, validated, processStatus, lang }: { edge: Edge; entries: GraphNode[]; dataTypes?: DataTypeDefinition[]; diagnostics: NodeGraphDiagnostic[]; validated: boolean; processStatus: ProcessStatus | null; lang: Language }): React.ReactElement {
  const { sourceType, targetType, sourcePort, targetPort, sourceIdl } = edgeContracts(entries, edge, dataTypes);
  const binding = entries.find((item) => sameNodePath(item.path, edge.target))?.node.inputs?.[String(edge.targetHandle).replace(/^in:/, '')];
  const desired = binding ? portInput(binding).desired_rate_hz : undefined;
  const declaration = sourcePort?.delivery_mode === 'declaration' && targetPort?.delivery_mode === 'declaration';
  const protocol = sourcePort?.contract;
  const units = [...new Set(Object.values(sourceIdl?.schema?.properties || {}).map((field) => field['x-unit']).filter(Boolean))];
  const issues = validated ? edgeInputDiagnostics({ ...edge, inputIndex: edge.data?.inputIndex, inputCount: edge.data?.inputCount }, diagnostics) : [];
  return <section className="node-graph-edge-idl"><h2>{declaration ? (lang === 'cn' ? '静态拓扑连接' : 'Static topology link') : (lang === 'cn' ? '实时数据连接' : 'Live data connection')}</h2><p><code>{edge.source}.{sourcePort?.name || '—'}</code> → <code>{edge.target}.{targetPort?.name || '—'}</code></p>{declaration ? <p>{lang === 'cn' ? '此连接只声明节点顺序，不传输数据。' : 'This link declares node order and carries no data.'}</p> : <><p>{lang === 'cn' ? '通道' : 'Channel'}: {sourcePort?.transport || '—'} · {lang === 'cn' ? '传送方式' : 'Delivery'}: {sourcePort?.delivery_mode || '—'}</p><p>{lang === 'cn' ? '源官方频率' : 'Official source frequency'}: {typeof sourcePort?.frequency_hz === 'number' ? `${sourcePort.frequency_hz} Hz` : '—'}</p><p>{lang === 'cn' ? '目标输入期望范围' : 'Destination input desired range'}: {desired?.min ?? '—'}–{desired?.max ?? '—'} Hz</p><p>{lang === 'cn' ? '协议' : 'Protocol'}: {protocol ? `${protocol.owner}/${protocol.id}@${protocol.version}` : '—'}</p><p>{lang === 'cn' ? '数据类型' : 'Payload type'}: {sourceType || '—'}{units.length ? ` · ${units.join(', ')}` : ''}</p>{sourceIdl && <p>{sourceIdl.codec} · {sourceIdl.media_type} · {sourceIdl.max_payload_bytes} bytes · {lang === 'cn' ? '字段' : 'Fields'}: {Object.keys(sourceIdl.schema?.properties || {}).join(', ') || '—'}</p>}</>}{sourceType !== targetType && <p role="alert">{lang === 'cn' ? '端口类型不匹配。' : 'Port types do not match.'}</p>}{issues.map((issue, index) => <p role="alert" key={index}>{issue.message}</p>)}<p>{lang === 'cn' ? '部署状态' : 'Deployment state'}: {processStatus?.state || '—'}</p></section>;
}
const DETAIL_FIELDS = ['frame_id', 'joint_names', 'adapter', 'health_timing', 'observation', 'movement', 'references', 'service_bindings'] as const;

export function settingValue(value: unknown, dataType: string): string {
  if (dataType === 'endpoint') return JSON.stringify(value ?? null, null, 2);
  if (dataType === 'string_array') return Array.isArray(value) ? value.join('\n') : '';
  if (dataType === 'string_table' || dataType === 'array' || dataType === 'object') return JSON.stringify(value ?? (dataType === 'array' ? [] : {}), null, 2);
  return value === undefined || value === null ? '' : String(value);
}
export function parseSetting(raw: string, dataType: string): unknown {
  if (dataType === 'boolean') { if (raw !== 'true' && raw !== 'false') throw new Error('Expected true or false.'); return raw === 'true'; }
  if (dataType === 'array' || dataType === 'object') { const parsed: unknown = JSON.parse(raw); if (dataType === 'array' ? !Array.isArray(parsed) : !parsed || Array.isArray(parsed) || typeof parsed !== 'object') throw new Error(`Expected a JSON ${dataType}.`); return parsed; }
  if (dataType === 'string_array') return raw.split('\n').map((part) => part.trim()).filter(Boolean);
  if (dataType === 'string_table') { const parsed: unknown = JSON.parse(raw); if (!parsed || Array.isArray(parsed) || typeof parsed !== 'object' || Object.values(parsed).some((value) => typeof value !== 'string')) throw new Error('Expected an object of strings.'); return parsed; }
  if (dataType === 'positive_integer' || dataType === 'nonnegative_integer') { const number = Number(raw); if (!Number.isInteger(number) || (dataType === 'positive_integer' ? number < 1 : number < 0)) throw new Error('Expected a valid integer.'); return number; }
  if (dataType === 'positive_number') { const number = Number(raw); if (!Number.isFinite(number) || number <= 0) throw new Error('Expected a positive number.'); return number; }
  if (dataType === 'number' || dataType === 'integer') { const number = Number(raw); if (!Number.isFinite(number) || (dataType === 'integer' && !Number.isInteger(number))) throw new Error(`Expected a valid ${dataType}.`); return number; }
  return raw;
}

export function adapterSettingDataType(name: string): 'positive_number' | 'string' {
  return name === 'frequency_hz' ? 'positive_number' : 'string';
}

function SettingEditor({ name, dataType, required, value, onChange, description, unit, min, max, choices, lang = 'en' }: { name: string; dataType: string; required: boolean; value: unknown; onChange: (value: unknown) => void; description?: string; unit?: string; min?: number; max?: number; choices?: string[]; lang?: Language }): React.ReactElement {
  const helpId = React.useId();
  const [draft, setDraft] = useState(() => settingValue(value, dataType));
  const [error, setError] = useState('');
  useEffect(() => { setDraft(settingValue(value, dataType)); setError(''); }, [value, dataType]);
  const commit = (): void => { try { if (!draft.trim() && !required) { onChange(undefined); setError(''); return; } const parsed = parseSetting(draft, dataType); if (typeof parsed === 'number' && (min !== undefined && parsed < min || max !== undefined && parsed > max)) throw new Error(`Expected ${min ?? '−∞'}…${max ?? '+∞'}.`); onChange(parsed); setError(''); } catch (issue) { setError(issue instanceof Error ? issue.message : String(issue)); } };
  const guidance = `${dataType}${unit ? ` · ${unit}` : ''}${description ? ` · ${description}` : ''}`;
  const effectiveChoices = dataType === 'boolean' ? ['true', 'false'] : choices;
  return <label className="node-graph-setting"><span>{name}{required ? ' *' : ''}</span>{dataType === 'endpoint' ? <><pre aria-describedby={helpId}>{draft}</pre><small>{lang === 'cn' ? '端点由归属方管理；请在归属方配置中修改。' : 'Owner-managed endpoint; edit the owning configuration.'}</small></> : effectiveChoices?.length ? <select aria-describedby={helpId} value={draft} onChange={(event) => { setDraft(event.target.value); onChange(dataType === 'boolean' ? event.target.value === 'true' : event.target.value); }}><option value="">—</option>{!effectiveChoices.includes(draft) && draft && <option value={draft}>{draft}</option>}{effectiveChoices.map((choice) => <option key={choice} value={choice}>{choice}</option>)}</select> : ['string_array', 'string_table', 'array', 'object'].includes(dataType) ? <textarea aria-describedby={helpId} rows={dataType === 'string_table' || dataType === 'object' ? 4 : 3} value={draft} onChange={(event) => setDraft(event.target.value)} onBlur={commit} /> : <input aria-describedby={helpId} type={dataType?.includes('integer') || dataType === 'positive_number' || dataType === 'number' ? 'number' : 'text'} step={dataType === 'positive_number' || dataType === 'number' ? 'any' : undefined} min={min} max={max} value={draft} onChange={(event) => setDraft(event.target.value)} onBlur={commit} onKeyDown={(event) => { if (event.key === 'Enter') event.currentTarget.blur(); }} />}<small id={helpId} className="node-graph-setting__metadata">{guidance}{min !== undefined || max !== undefined ? ` · ${min ?? '−∞'}…${max ?? '+∞'}` : ''}</small>{error && <small role="alert" className="node-graph-error">{error}</small>}</label>;
}

function HardwareFields({ node, path, definition, options, optionsCurrent = true, adapterPreviewAvailable = false, lang, onChange, mode = 'all' }: { node: NodeInstance; path: string; definition: NodeTypeDefinition; options: NodeOptions | null; optionsCurrent?: boolean; adapterPreviewAvailable?: boolean; lang: Language; onChange: (change: Partial<NodeInstance>) => void; mode?: 'all' | 'basic' | 'advanced' }): React.ReactElement {
  const adapter = node.adapter || { type: '', settings: {} };
  const adapterSettings = adapter.settings || {};
  const setAdapter = (key: string, value: unknown): void => {
    if (key === 'service_instance') {
      onChange({ adapter: { ...adapter, settings: { service_instance: value } } });
      return;
    }
    if (key === 'device_id') {
      const choice = settingOptionGroup(options, path, 'adapter.settings.device_id')?.options.find((item) => item.value === value && item.available);
      const profile = choice?.profile;
      if (profile && (!adapter.type || profile.adapter?.type === adapter.type)) {
        onChange({ adapter: { ...adapter, ...profile.adapter, settings: {
          ...adapterSettings, ...profile.adapter?.settings, service_instance: adapterSettings.service_instance,
        } }, frame_id: profile.frame_id, capabilities: profile.capabilities,
          health_timing: profile.health_timing, observation: profile.observation,
          movement: undefined, joint_names: undefined });
        return;
      }
    }
    onChange({ adapter: { ...adapter, settings: { ...adapterSettings, [key]: value } } });
  };
  const adapterTypeGroup = settingOptionGroup(options, path, 'adapter.type');
  const adapterSettingGroups = (options?.setting_options || []).filter((group) => group.node_path === path && group.setting.startsWith('adapter.settings.'));
  const adapterSettingNames = [...new Set([
    ...Object.keys(adapterSettings).filter((key) => key !== 'socket_path'),
    ...adapterSettingGroups.map((group) => group.setting.slice('adapter.settings.'.length)),
    ...(adapter.package ? ['service_instance'] : []),
    ...(definition.kind === 'camera' && definition.package ? ['service_instance', 'device_id', 'frequency_hz'] : []),
  ])].sort();
  const timing = node.health_timing;
  const observation = node.observation;
  const movement = node.movement;
  const limits = definition.kind === 'base' ? ['max_linear_m_s', 'max_angular_rad_s', 'max_velocity_lease_ms'] : definition.kind === 'manipulator' ? ['max_joint_position_rad', 'max_joint_speed_rad_s'] : [];
  return <div className="node-graph-hardware-fields">
    {mode !== 'advanced' && <>
    <h3>Adapter</h3>
    {adapterTypeGroup ? <CatalogSelector group={adapterTypeGroup} disabled={!optionsCurrent && !adapterPreviewAvailable} label={lang === 'cn' ? '适配器族' : 'Adapter family'} value={adapter.type} lang={lang} onChange={(value) => onChange({ adapter: { ...adapter, type: value, ...(definition.package ? { package: definition.package } : {}) } })} /> : <p role="status" className={options ? 'node-graph-error' : undefined}>{options ? (lang === 'cn' ? '适配器选项不可用。请检查 EOS 服务。' : 'Adapter options are unavailable. Check the EOS service.') : (lang === 'cn' ? '正在加载适配器选项…' : 'Loading adapter choices…')}</p>}
    {adapterSettingNames.map((key) => {
      const group = settingOptionGroup(options, path, `adapter.settings.${key}`);
      return group ? <CatalogSelector key={key} group={group} disabled={!optionsCurrent} pin={adapter.package} label={key} value={String(adapterSettings[key] || '')} lang={lang} onChange={(value) => setAdapter(key, value)} /> : definition.kind === 'camera' && definition.package && (key === 'service_instance' || key === 'device_id') ? <p key={key} role="status" className={options ? 'node-graph-error' : undefined}>{options ? (lang === 'cn' ? `${key} 的已验证选项不可用。请检查软件包配置。` : `Verified ${key} choices are unavailable. Check package configuration.`) : (lang === 'cn' ? `正在加载 ${key} 选项…` : `Loading ${key} choices…`)}</p> : <SettingEditor key={key} name={key} dataType={adapterSettingDataType(key)} required={key === 'service_instance' || (key === 'frequency_hz' && definition.kind === 'camera' && !!definition.package)} max={key === 'frequency_hz' ? 1000 : undefined} value={adapterSettings[key]} onChange={(value) => setAdapter(key, value)} />;
    })}
    <SettingEditor name="frame_id" dataType="string" required value={node.frame_id} onChange={(value) => onChange({ frame_id: String(value) })} />
    {definition.kind === 'manipulator' && <SettingEditor name="joint_names" dataType="string_array" required value={node.joint_names} onChange={(value) => onChange({ joint_names: value as string[] })} />}
    </>}
    {mode !== 'basic' && <>
    <h3>Health timing</h3>
    {(['poll_interval_ms', 'stale_after_ms'] as const).map((key) => <SettingEditor key={`${node.id}:${key}`} name={key} dataType="positive_integer" required unit="ms" value={timing?.[key]} onChange={(value) => onChange({ health_timing: { ...timing, [key]: value as number } as NodeInstance['health_timing'] })} />)}
    <h3>Observation</h3>
    <label>max_rate_hz<small>number · Hz</small><input type="number" min={0.1} step="0.1" value={observation?.max_rate_hz ?? ''} onChange={(event) => { const value = Number(event.target.value); if (Number.isFinite(value) && value > 0) onChange({ observation: { ...observation, max_rate_hz: value } as NodeInstance['observation'] }); }} /></label>
    <SettingEditor name="max_payload_bytes" dataType="positive_integer" required unit="bytes" value={observation?.max_payload_bytes} onChange={(value) => onChange({ observation: { ...observation, max_payload_bytes: value as number } as NodeInstance['observation'] })} />
    {limits.length > 0 && <><h3>Movement limits</h3><SettingEditor name="modes" dataType="string_array" required value={movement?.modes} onChange={(value) => onChange({ movement: { ...movement, modes: value as string[] } as NodeInstance['movement'] })} />{limits.map((key) => <label key={key}>{key}<small>number · {key.endsWith('_ms') ? 'ms' : key.includes('rad') ? 'rad or rad/s' : 'm/s'}</small><input type="number" min={0} step="any" value={movement?.limits[key] ?? ''} onChange={(event) => { const value = Number(event.target.value); if (Number.isFinite(value) && value >= 0) onChange({ movement: { ...movement, limits: { ...movement?.limits, [key]: value } } as NodeInstance['movement'] }); }} /></label>)}</>}
    </>}
  </div>;
}

function PortRateEditor({ entry, entries, lang, onChange }: { entry: GraphNode; entries: GraphNode[]; lang: Language; onChange: (input: string, rate: { min?: number; max?: number }) => void }): React.ReactElement {
  const [drafts, setDrafts] = useState<Record<string, { min: string; max: string }>>({});
  const [error, setError] = useState<Record<string, string>>({});
  const cn = lang === 'cn';
  const bindings = Object.entries(entry.node.inputs || {});
  useEffect(() => { setDrafts(Object.fromEntries(bindings.map(([name, binding]) => { const rate = portInput(binding).desired_rate_hz; return [name, { min: rate?.min === undefined ? '' : String(rate.min), max: rate?.max === undefined ? '' : String(rate.max) }]; }))); setError({}); }, [entry.node.inputs]);
  const commit = (name: string): void => {
    const draft = drafts[name]; if (!draft) return;
    const min = draft.min.trim() ? Number(draft.min) : undefined;
    const max = draft.max.trim() ? Number(draft.max) : undefined;
    if (min !== undefined && (!Number.isFinite(min) || min <= 0) || max !== undefined && (!Number.isFinite(max) || max <= 0) || min !== undefined && max !== undefined && min > max) {
      setError((current) => ({ ...current, [name]: cn ? '期望频率必须为有限正数，且最小值不能大于最大值。' : 'Desired rates must be finite and positive, with minimum no greater than maximum.' })); return;
    }
    setError((current) => ({ ...current, [name]: '' })); onChange(name, { ...(min === undefined ? {} : { min }), ...(max === undefined ? {} : { max }) });
  };
  return <section className="node-graph-port-rates"><h3>{cn ? '输入端口期望频率' : 'Input port desired rates'}</h3>{bindings.map(([name, binding]) => <fieldset key={name}><legend>{name}</legend><p>{cn ? '源官方频率' : 'Official source frequency'}: {inputSources(binding).map((source) => { const producer = entries.find((item) => sameNodePath(item.path, source.node)); const rate = producer?.definition?.outputs.find((port) => port.name === source.output)?.frequency_hz; return `${source.node}.${source.output}: ${typeof rate === 'number' ? `${rate} Hz` : '—'}`; }).join(', ')}</p>{(['min', 'max'] as const).map((bound) => <label key={bound}>{bound === 'min' ? cn ? '期望最小值（Hz）' : 'Desired minimum (Hz)' : cn ? '期望最大值（Hz）' : 'Desired maximum (Hz)'}<input type="number" min="0" step="any" value={drafts[name]?.[bound] ?? ''} onChange={(event) => setDrafts((current) => ({ ...current, [name]: { ...current[name], [bound]: event.target.value } }))} onBlur={() => commit(name)} onKeyDown={(event) => { if (event.key === 'Enter') event.currentTarget.blur(); }} /></label>)}{error[name] && <p role="alert" className="node-graph-error">{error[name]}</p>}</fieldset>)}</section>;
}

function NodeInspector({ entry, entries, dataTypes, lang, options, optionsCurrent = true, optionsFailed = false, adapterPreviewAvailable = false, runId, activeRevision, currentRun, onChange, onBind, onRename, onRemove }: { entry: GraphNode; entries: GraphNode[]; dataTypes: DataTypeDefinition[]; lang: Language; options: NodeOptions | null; optionsCurrent?: boolean; optionsFailed?: boolean; adapterPreviewAvailable?: boolean; runId: string | null; activeRevision: string | null; currentRun: boolean; onChange: (change: Partial<NodeInstance>) => void; onBind: (target: string) => void; onRename: (id: string) => void; onRemove: () => void }): React.ReactElement {
  const copy = COPY[lang];
  const [id, setId] = useState(entry.node.id);
  const [raw, setRaw] = useState('');
  const [error, setError] = useState('');
  const bindingTarget = entries.find((target) => { const value = target.node.inputs?.command; return value && inputSources(value).some((source) => sameNodePath(source.node, entry.path)); })?.path || '';
  useEffect(() => { setId(entry.node.id); setRaw(JSON.stringify(Object.fromEntries(Object.entries(entry.node).filter(([key]) => (DETAIL_FIELDS as readonly string[]).includes(key))), null, 2)); setError(''); }, [entry.path, entry.node]);
  const commitDetails = (): void => { try { const parsed: unknown = JSON.parse(raw); if (!parsed || Array.isArray(parsed) || typeof parsed !== 'object' || Object.keys(parsed).some((key) => !(DETAIL_FIELDS as readonly string[]).includes(key))) throw new Error(copy.invalid); const details = parsed as Partial<NodeInstance>; for (const field of DETAIL_FIELDS) if (!(field in details) && field in entry.node) Object.assign(details, { [field]: undefined }); onChange(details); setError(''); } catch { setError(copy.invalid); } };
  return <section className="node-graph-inspector">
    <h2>{copy.inspector}</h2>{(entry.definition?.lifecycle === 'adapter' || entry.definition?.settings.some((field) => field.selector)) && !optionsCurrent && <p role={optionsFailed ? 'alert' : 'status'}>{optionsFailed ? (lang === 'cn' ? '选项刷新失败。请检查拥有方服务并刷新。' : 'Choice refresh failed. Check the owner service and refresh.') : (lang === 'cn' ? '选项正在刷新；旧选项暂不可用。' : 'Refreshing choices; previous choices are temporarily unavailable.')}</p>}<p><strong>{lang === 'cn' ? '类型描述' : 'Type description'}:</strong> {entry.definition?.description || (lang === 'cn' ? '当前目录没有类型描述。' : 'No type description in this catalog.')}</p><p><strong>{copy.type}:</strong> {entry.node.node_type}</p><p><strong>{copy.lifecycle}:</strong> {entry.definition?.lifecycle || '—'}</p>
    <details className="node-graph-port-details"><summary>{lang === 'cn' ? '端口与数据契约' : 'Ports and data contracts'}</summary>{([['input', entry.definition?.inputs], ['output', entry.definition?.outputs]] as const).map(([direction, ports]) => <section key={direction}><h3>{direction === 'input' ? (lang === 'cn' ? '输入' : 'Inputs') : (lang === 'cn' ? '输出' : 'Outputs')}</h3><ul>{ports?.map((port) => { const payload = dataTypes.find((item) => item.id === port.data_type); const fields = Object.keys(payload?.schema?.properties || {}); const units = [...new Set(Object.values(payload?.schema?.properties || {}).map((field) => field['x-unit']).filter(Boolean))]; return <li key={port.name}><code>{port.name}</code> · {port.data_type}{port.description ? ` · ${port.description}` : ''}{port.transport ? ` · ${port.transport}` : ''}{port.delivery_mode ? ` · ${port.delivery_mode}` : ''}{typeof port.frequency_hz === 'number' ? ` · ${port.frequency_hz} Hz` : ''}{port.contract ? ` · ${port.contract.owner}/${port.contract.id}@${port.contract.version}` : ''}{payload ? ` · ${payload.codec} · ${payload.media_type} · ${payload.max_payload_bytes} bytes · ${lang === 'cn' ? '字段' : 'fields'}: ${fields.join(', ') || '—'}` : ''}{units.length ? ` · ${units.join(', ')}` : ''}</li>; })}</ul></section>)}</details>
    <details className="node-graph-debug-details"><summary>{lang === 'cn' ? '调试数据（所有节点）' : 'Debug data (all nodes)'}</summary><p>{lang === 'cn' ? '当前图中的节点实例。未填写的字段保持未设置；类型声明与软件包来源如下。' : 'Node instance in the current graph. Omitted fields remain unset; the type declaration and package source follow.'}</p><h3>{lang === 'cn' ? '节点实例' : 'Node instance'}</h3><pre aria-label={lang === 'cn' ? '节点实例 JSON' : 'Node instance JSON'}>{JSON.stringify(entry.node, null, 2)}</pre><h3>{lang === 'cn' ? '类型声明' : 'Type declaration'}</h3><pre aria-label={lang === 'cn' ? '类型声明 JSON' : 'Type declaration JSON'}>{entry.definition ? JSON.stringify(entry.definition, null, 2) : (lang === 'cn' ? '所选目录中没有类型声明。' : 'No type declaration in the selected catalog.')}</pre></details>
    <label>{copy.id}<input value={id} onChange={(event) => setId(event.target.value.toLowerCase())} onBlur={() => { if (id !== entry.node.id) onRename(id.trim()); }} onKeyDown={(event) => { if (event.key === 'Enter') event.currentTarget.blur(); }} /></label>
    <label>{copy.label}<input value={entry.node.label} onChange={(event) => onChange({ label: event.target.value })} /></label>
    <label>{lang === 'cn' ? '实例描述' : 'Instance description'}<textarea rows={2} value={entry.node.description} onChange={(event) => onChange({ description: event.target.value })} /></label>
    <h3>{copy.capabilities}</h3><div className="node-graph-capabilities">{entry.definition?.capabilities.map((capability) => <label key={capability}><input type="checkbox" checked={entry.node.capabilities?.includes(capability) || false} onChange={(event) => onChange({ capabilities: event.target.checked ? [...(entry.node.capabilities || []), capability] : (entry.node.capabilities || []).filter((item) => item !== capability) })} />{capability}</label>)}</div>
    {(/^(?:eos\.dispatcher|isos\.eos_dispatcher)\.primitive_/.test(entry.node.node_type)) && <section className="node-graph-binding"><h3>{lang === 'cn' ? '原语绑定' : 'Primitive binding'}</h3><p className="node-graph-card__command-route">{lang === 'cn' ? 'UI → EOS · 直接类型化命令，经 EOS Dispatcher 准入；此控制端点不是图端口。' : 'UI → EOS · direct typed command through EOS Dispatcher admission; this control endpoint is not a graph port.'}</p><p>{options?.primitive_bindings.find((option) => option.node_type === entry.node.node_type)?.reason || ''}</p><label>{lang === 'cn' ? '目标硬件' : 'Target hardware'}<select value={bindingTarget} onChange={(event) => onBind(event.target.value)}><option value="">—</option>{bindingTarget && !options?.targets.some((target) => sameNodePath(target.path, bindingTarget)) && <option value={bindingTarget}>{bindingTarget}</option>}{options?.targets.filter((target) => target.device_kind === (entry.node.node_type.includes('_base') ? 'base' : 'manipulator')).map((target) => <option key={target.path} value={target.path} disabled={!target.available}>{target.path}{target.available ? '' : ` — ${target.reason || 'unavailable'}`}</option>)}</select></label>{!options && <p role="status">{lang === 'cn' ? '节点选项服务不可用' : 'Node options service unavailable'}</p>}</section>}

    {!!entry.definition?.settings.length && <><h3>{copy.settings}</h3>{entry.definition.settings.map((field) => {
      const group = settingOptionGroup(options, entry.path, field.name);
      const update = (value: unknown): void => {
        const settings = { ...entry.node.settings };
        if (value === undefined) delete settings[field.name]; else settings[field.name] = value;
        onChange({ settings });
      };
      if (field.selector?.provider === 'ui') return <p key={`${entry.path}:${field.name}`} role="alert" className="node-graph-error">{lang === 'cn' ? '当前图不支持浏览器输入选择器。' : 'Browser input selectors are unsupported by the current graph.'}</p>;
      if (group) return <CatalogSelector key={`${entry.path}:${field.name}`} disabled={!optionsCurrent} group={group} pin={entry.node.package} help={field.description} label={field.name} value={String(entry.node.settings?.[field.name] ?? field.default ?? '')} lang={lang} onChange={update} />;
      if (field.selector) return <div key={`${entry.path}:${field.name}`} className="node-graph-catalog-selector"><strong>{field.name}</strong>{field.description && <small>{field.description}</small>}<p role="status" className="node-graph-error">{lang === 'cn' ? '此节点的可用选项尚未就绪。请检查拥有方服务，然后刷新选项。' : 'Options for this node are unavailable. Check the owner service, then refresh options.'}</p></div>;
      return <SettingEditor key={`${entry.path}:${field.name}`} lang={lang} name={field.name} dataType={field.data_type} required={field.required} description={field.description} unit={field.unit} min={field.min} max={field.max} choices={field.choices} value={entry.node.settings?.[field.name] ?? field.default} onChange={update} />;
    })}</>}
    {entry.definition?.lifecycle === 'adapter' && <HardwareFields node={entry.node} path={entry.path} definition={entry.definition} options={options} optionsCurrent={optionsCurrent} adapterPreviewAvailable={adapterPreviewAvailable} lang={lang} onChange={onChange} />}
    {entry.definition?.lifecycle === 'adapter' && <><h3>{copy.details}</h3><textarea rows={12} value={raw} onChange={(event) => setRaw(event.target.value)} /><button type="button" onClick={commitDetails}>{copy.applyDetails}</button></>}
    {error && <p role="alert" className="node-graph-error">{error}</p>}<button type="button" className="node-graph-remove" onClick={onRemove}>{copy.remove}</button>
  </section>;
}

export function NodeGraphManager({ lang, visible = true, panelMenu, onDirtyChange, onNoticesChange, onOpenPanel, onOpenPage, onStopIntent, onStopOutcome }: Props): React.ReactElement {
  const copy = COPY[lang];
  const [graph, setGraph] = useState<NodeGraph | null>(null);
  const [rawGraph, setRawGraph] = useState<unknown>(null);
  const [saved, setSaved] = useState('');
  const [revision, setRevision] = useState('');
  const [readOnlyReason, setReadOnlyReason] = useState<string | null>(null);
  const [types, setTypes] = useState<NodeTypeDefinition[]>([]);
  const [adapterCatalog, setAdapterCatalog] = useState<NodeTypeCatalog | null>(null);
  const [dataTypes, setDataTypes] = useState<DataTypeDefinition[]>([]);
  const [processStatus, setProcessStatus] = useState<ProcessStatus | null>(null);
  const [selected, setSelected] = useState<string[]>([]);
  const [inspectorTarget, setInspectorTarget] = useState<InspectorTarget>('none');
  const [typeId, setTypeId] = useState('');
  const [libraryView, setLibraryView] = useState<'nodes' | 'packages'>('nodes');
  const [installedPackages, setInstalledPackages] = useState<InstalledPackage[]>([]);
  const [selectedPackagePin, setSelectedPackagePin] = useState('');
  const [packageConfigDirty, setPackageConfigDirty] = useState(false);
  const [newId, setNewId] = useState('');
  const [quickSearch, setQuickSearch] = useState('');
  const [quickAdd, setQuickAdd] = useState<{ left: number; top: number; x: number; y: number } | null>(null);
  const [quickDetail, setQuickDetail] = useState<string>('');
  const [selectedEdge, setSelectedEdge] = useState<string>('');
  const [edgeMenu, setEdgeMenu] = useState<{ left: number; top: number; id: string } | null>(null);
  const [libraryCollapsed, setLibraryCollapsed] = useState(false);
  const [sideCollapsed, setSideCollapsed] = useState(false);
  const [showOverview, setShowOverview] = useState(false);
  const [flowInstance, setFlowInstance] = useState<ReactFlowInstance | null>(null);
  const canvasRef = useRef<HTMLDivElement>(null);
  const quickSearchRef = useRef<HTMLInputElement>(null);
  const overviewTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [undoStack, setUndoStack] = useState<NodeGraph[]>([]);
  const [redoStack, setRedoStack] = useState<NodeGraph[]>([]);
  const [diagnostics, setDiagnostics] = useState<NodeGraphDiagnostic[]>([]);
  const [warnings, setWarnings] = useState<NodeGraphDiagnostic[]>([]);
  const [validationStamp, setValidationStamp] = useState<DraftValidationStamp | null>(null);
  const [validatedEdges, setValidatedEdges] = useState<ValidatedEdge[]>([]);
  const [edgeInputInvalid, setEdgeInputInvalid] = useState(false);
  const [diagnosticsOpen, setDiagnosticsOpen] = useState(false);
  const [frontNode, setFrontNode] = useState<string | null>(null);
  const setMessage = (text: string, error = false): void => { publishUiMessage(text, error ? 'error' : 'info'); };
  const [optionSnapshot, setOptionSnapshot] = useState<OptionSnapshot | null>(null);
  const [optionErrorDraft, setOptionErrorDraft] = useState('');
  const optionRequest = useRef(0);
  const catalogReloadRequest = useRef(0);
  const statusRequest = useRef(0);
  const statusStreamReady = useRef(false);
  const readinessRequest = useRef(0);
  const lastDeploymentFailure = useRef('');
  const graphForCatalog = useRef(graph);
  const revisionForCatalog = useRef(revision);
  graphForCatalog.current = graph;
  revisionForCatalog.current = revision;
  const [packageGeneration, setPackageGeneration] = useState(0);
  const [busy, setBusy] = useState(false);
  const [stopping, setStopping] = useState(false);
  const [advancedOpen, setAdvancedOpen] = useState(false);
  const [readiness, setReadiness] = useState<CoreReadiness | null>(null);
  useEffect(() => {
    const failedOwners = processStatus?.state === 'degraded' ? processStatus.processes
      .filter((process) => !['running', 'ready'].includes(process.state || ''))
      .map((process) => `${process.id}: ${process.error || process.last_error || process.health?.error || process.state || 'unavailable'}`)
      .join('; ') : '';
    const failure = [processStatus?.last_error, failedOwners].filter(Boolean).join(' — ') || (processStatus?.state === 'degraded' ? 'Deployment is degraded; check owner process status.' : '');
    const key = failure ? `${processStatus?.run_id || 'unknown'}:${failure}` : '';
    if (key && key !== lastDeploymentFailure.current) browserLog('error', `Deployment ${processStatus?.run_id || 'unknown'}: ${failure}`);
    lastDeploymentFailure.current = key;
  }, [processStatus?.run_id, processStatus?.state, processStatus?.last_error]);
  const [graphLoadError, setGraphLoadError] = useState('');
  const [newEmptyDraft, setNewEmptyDraft] = useState(false);
  const [poolEntries, setPoolEntries] = useState<GraphPoolEntry[]>([]);
  const [poolName, setPoolName] = useState('');
  const [poolSelected, setPoolSelected] = useState('');
  const [poolRevision, setPoolRevision] = useState('');
  const [fallbackNotice, setFallbackNotice] = useState('');
  const poolFileExists = poolEntries.some((entry) => entry.name === poolName);
  const [viewLost, setViewLost] = useState(false);
  const initialFitDone = useRef(false);
  const layoutKeyRef = useRef('');
  const editGeneration = useRef(0);
  const activeGesture = useRef(false);
  const [positions, setPositions] = useState<NodePositions>({});
  const [panelWidths, setPanelWidths] = useState<GraphPanelWidths>(DEFAULT_GRAPH_PANEL_WIDTHS);
  const [viewportWidth, setViewportWidth] = useState(window.innerWidth);
  const panelResize = useRef<{ panel: keyof GraphPanelWidths; x: number; width: number } | null>(null);
  const [nodeSizes, setNodeSizes] = useState<NodeSizes>({});
  const [mirrors, setMirrors] = useState<NodeMirrors>({});
  const [viewOpen, setViewOpen] = useState(false);
  useEffect(() => { const resize = (): void => setViewportWidth(window.innerWidth); window.addEventListener('resize', resize); return () => window.removeEventListener('resize', resize); }, []);
  const dirty = Boolean(graph && JSON.stringify(graph) !== saved);
  const readOnly = readOnlyReason !== null;
  const draftKey = graph ? JSON.stringify(graph) : '';
  const currentDraft = useRef(draftKey);
  currentDraft.current = draftKey;
  const nodeOptions = optionSnapshot?.draft === draftKey ? optionSnapshot.options : null;
  const previewOptions = graph && adapterCatalog ? draftAdapterOptions(graph, adapterCatalog) : null;
  const displayOptions = displayNodeOptions(nodeOptions, optionSnapshot?.options || null, previewOptions);
  const optionRefreshing = !!graph && !nodeOptions && optionErrorDraft !== draftKey;
  useEffect(() => {
    const requestId = ++optionRequest.current;
    if (!graph || readOnly) return;
    const draft = JSON.stringify(graph);
    const controller = new AbortController();
    setOptionErrorDraft('');
    const timeout = window.setTimeout(() => controller.abort(), 35000);
    const timer = window.setTimeout(() => { void getNodeOptions(graph, controller.signal).then((options) => {
      if (optionResponseIsCurrent(requestId, optionRequest.current, draft, currentDraft.current)) { setOptionSnapshot({ draft, options }); setOptionErrorDraft(''); }
    }).catch(() => {
      if (!optionResponseIsCurrent(requestId, optionRequest.current, draft, currentDraft.current)) return;
      if (adapterCatalog) setOptionSnapshot({ draft, options: draftAdapterOptions(graph, adapterCatalog) });
      setOptionErrorDraft(draft);
    }).finally(() => window.clearTimeout(timeout)); }, 250);
    return () => { window.clearTimeout(timer); window.clearTimeout(timeout); controller.abort(); optionRequest.current += 1; };
  }, [graph, readOnly, packageGeneration, adapterCatalog]);
  useEffect(() => { onDirtyChange?.(dirty); }, [dirty, onDirtyChange]);
  const entries = useMemo(() => graph ? graphNodes(graph, types) : [], [graph, types]);
  const monitorRateHz = useMemo(() => { try { return monitorRefreshHz(saved ? JSON.parse(saved) as NodeGraph : null); } catch { return DEFAULT_MONITOR_REFRESH_HZ; } }, [saved]);
  const monitorReading = useArmMonitor(visible && entries.some((entry) => entry.node.node_type === 'isos.ui.monitor.arm@v0.2'), monitorRateHz);
  const selectorBlocked = catalogSelectionBlocks(graph?.version, entries, optionErrorDraft === draftKey ? null : nodeOptions);
  const stableCatalogError = catalogErrorIsStable(selectorBlocked && (optionErrorDraft === draftKey || unavailableSelections(nodeOptions).length > 0), draftKey, optionSnapshot, optionErrorDraft);
  const catalogNotice = stableCatalogError ? (lang === 'cn' ? '当前草稿的目录选项不可用。检查图仍可使用；修复选项后才能保存或运行。' : 'Catalog options for this draft are unavailable. Check graph remains available; repair the choice before saving or running.') : null;
  useEffect(() => {
    const notices = [
      catalogNotice && { text: catalogNotice, alert: true },
      fallbackNotice && { text: fallbackNotice, alert: false },
      readOnlyReason && { text: `${lang === 'cn' ? '只读节点图：' : 'Read-only graph: '}${readOnlyReason}`, alert: true },
      viewLost && { text: lang === 'cn' ? '节点可能在视野之外。使用“显示全图”恢复。' : 'Nodes may be outside the view. Use Fit graph to recover.', alert: false },
      processStatus?.last_error && { text: processStatus.last_error, alert: true },
    ].filter((notice): notice is { text: string; alert: boolean } => Boolean(notice));
    onNoticesChange?.(notices);
  }, [catalogNotice, fallbackNotice, readOnlyReason, viewLost, processStatus?.last_error, lang, onNoticesChange]);
  useEffect(() => () => onNoticesChange?.([]), [onNoticesChange]);
  useEffect(() => { if (graph) browserLog('debug', `Graph revision ${revision.slice(0, 8) || 'unsaved'}: ${entries.length} nodes; visible=${visible}`); }, [graph, revision, visible]);
  useEffect(() => {
    if (!visible || !flowInstance || !entries.length || initialFitDone.current) return;
    let frame = 0;
    let cancelled = false;
    let attempts = 0;
    const fitWhenReady = (): void => {
      if (cancelled || initialFitDone.current) return;
      if (activeGesture.current) { initialFitDone.current = true; return; }
      const canvas = canvasRef.current;
      const measured = flowInstance.getNodes().some((node) => Boolean(node.width && node.height));
      if (!canvas?.clientWidth || !canvas.clientHeight || !measured) {
        if (++attempts < 120) frame = requestAnimationFrame(fitWhenReady);
        else browserLog('warn', 'Graph initial fit skipped because the canvas or nodes were not measured');
        return;
      }
      initialFitDone.current = true;
      void flowInstance.fitView({ padding: 0.15 });
    };
    frame = requestAnimationFrame(fitWhenReady);
    return () => { cancelled = true; cancelAnimationFrame(frame); };
  }, [visible, flowInstance, entries.length]);
  useEffect(() => {
    if (!visible || !flowInstance || !entries.length) return;
    const timer = window.setTimeout(() => {
      const canvas = canvasRef.current;
      if (!canvas || !canvas.clientWidth || !canvas.clientHeight) return;
      const bounds = canvas.getBoundingClientRect();
      const cards = canvas.querySelectorAll('.react-flow__node');
      const onScreen = Array.from(cards).some((card) => {
        const rect = card.getBoundingClientRect();
        return rect.right > bounds.left && rect.left < bounds.right && rect.bottom > bounds.top && rect.top < bounds.bottom;
      });
      if (cards.length && !onScreen) {
        browserLog('warn', `Graph nodes outside canvas; ${entries.length} graph nodes, ${cards.length} rendered nodes, viewport ${JSON.stringify(flowInstance.getViewport())}`);
        setViewLost(true);
      } else if (onScreen) {
        setViewLost(false);
      }
    }, 100);
    return () => window.clearTimeout(timer);
  }, [graph, visible, flowInstance, entries.length]);
  const availableTypes = useMemo(() => addableTypes(types), [types]);
  const selectedPackage = installedPackages.find((entry) => installedPin(entry) === selectedPackagePin);
  const selectedLibraryType = availableTypes.find((type) => catalogTypeKey(type) === typeId);
  const selectLibraryType = (key: string): void => { setTypeId(key); setSelected([]); setSelectedEdge(''); setInspectorTarget(key ? 'type' : 'none'); setSideCollapsed(false); };
  const quickTypes = useMemo(() => availableTypes.filter((type) => !(type.package && type.implementation?.kind === 'managed_process') && `${type.id} ${type.core} ${type.layer} ${type.group} ${type.kind} ${type.package?.id || ''} ${type.package?.version || ''}`.toLowerCase().includes(quickSearch.toLowerCase())), [availableTypes, quickSearch]);
  useEffect(() => { if (quickAdd) quickSearchRef.current?.focus(); }, [quickAdd]);
  useEffect(() => { if (diagnostics.length) setDiagnosticsOpen(true); }, [diagnostics]);
  useEffect(() => () => { if (overviewTimer.current) clearTimeout(overviewTimer.current); }, []);
  const revealOverview = (): void => {
    setShowOverview(true);
    if (overviewTimer.current) clearTimeout(overviewTimer.current);
    overviewTimer.current = setTimeout(() => setShowOverview(false), 1800);
  };
  const chosen = selected.length === 1 ? entries.find((entry) => entry.path === selected[0]) : undefined;
  const refreshLibraries = (): void => { window.dispatchEvent(new Event('isosui:packages-changed')); };
  const refreshPool = async (): Promise<void> => {
    try { setPoolEntries((await getGraphPool()).graphs); }
    catch (error) { setMessage(error instanceof Error ? error.message : String(error), true); }
  };
  const loadPoolGraph = async (name = poolSelected): Promise<void> => {
    if (!name) return;
    if (dirty && !window.confirm(copy.discard)) return;
    setBusy(true);
    try {
      const [entry, snapshotResult] = await Promise.all([
        getGraphPoolEntry(name),
        getGraphCatalogSnapshot().then((value) => ({ value })).catch(() => null),
      ]);
      if (!nodeGraphHasEditableShape(entry.graph) || !['0.2.0', '0.2.1'].includes(entry.graph.version)) throw new Error('Selected pool graph cannot be opened.');
      const snapshot = snapshotResult?.value;
      const global = snapshot ? snapshot.global : await getNodeTypes().catch(() => null);
      const catalog = combineNodeTypeCatalogs(snapshot?.selected || null, global);
      const sameAsSaved = snapshot && JSON.stringify(entry.graph) === JSON.stringify(snapshot.graph.graph);
      const poolReadOnlyReason = entry.graph.version === '0.2.0' ? (lang === 'cn' ? '旧版 0.2.0 图；可预览转换草稿。' : 'Legacy 0.2.0 graph; preview a conversion draft.') : sameAsSaved
        ? graphCatalogReadOnlyReason(snapshot)
        : snapshot ? snapshot.reason || nodeGraphReadOnlyReason({ ...snapshot.graph, graph: entry.graph, read_only: false, diagnostics: [] }, catalog.types)
          : (lang === 'cn' ? '部署所有者不可用；可预览图库文件。修复软件包后重新载入以编辑和运行。' : 'Deployment owner is unavailable; this graph file can be previewed. Repair the package and reload to edit or run.');
      editGeneration.current += 1;
      catalogReloadRequest.current += 1;
      setGraph(entry.graph); setRawGraph(null); setRevision(entry.revision); setSaved(JSON.stringify(entry.graph));
      setNewEmptyDraft(false); setReadOnlyReason(poolReadOnlyReason); setGraphLoadError('');
      setTypes(catalog.types); setDataTypes(catalog.data_types || []); setAdapterCatalog(catalog);
      setDiagnostics([]); setWarnings([]); setValidatedEdges([]); setValidationStamp(null);
      setSelected([]); setSelectedEdge(''); setInspectorTarget('none'); setUndoStack([]); setRedoStack([]);
      setPositions(parseNodePositions(localStorage.getItem(graphLayoutKey(entry.graph))));
      setPanelWidths(parseGraphPanelWidths(localStorage.getItem(graphPanelWidthsKey(entry.graph))));
      setMirrors(parseNodeMirrors(localStorage.getItem(graphMirrorKey(entry.graph))));
      layoutKeyRef.current = graphLayoutKey(entry.graph);
      setOptionSnapshot(null);
      setPoolRevision(entry.revision);
      setPoolName(entry.name);
      setPoolSelected(entry.name);
      setFallbackNotice('');
      void getCoreReadiness().then(setReadiness).catch(() => undefined);
      localStorage.setItem('isosui-graph-pool-last-name', entry.name);
      setMessage(lang === 'cn' ? `已打开图库文件“${entry.name}”。` : `Opened graph file “${entry.name}”.`);
    } catch (error) { setMessage(error instanceof Error ? error.message : String(error), true); }
    finally { setBusy(false); }
  };
  const previewGraphConversion = async (): Promise<void> => {
    if (!graph || graph.version !== '0.2.0' || !poolName || busy) return;
    setBusy(true);
    try {
      const entry = await getGraphPoolEntry(poolName);
      if (entry.revision !== poolRevision || JSON.stringify(entry.graph) !== JSON.stringify(graph)) throw new Error(copy.stale);
      const converted = { ...entry.graph, version: '0.2.1' };
      const catalog = await getNodeTypes();
      const reason = nodeGraphReadOnlyReason({ revision: entry.revision, graph: converted }, catalog.types);
      if (reason) throw new Error(reason);
      setGraph(converted); setTypes(catalog.types); setDataTypes(catalog.data_types || []); setAdapterCatalog(catalog);
      setReadOnlyReason(null); setSaved(JSON.stringify(entry.graph));
      setDiagnostics([]); setWarnings([]); setValidatedEdges([]); setValidationStamp(null);
      setMessage(lang === 'cn' ? '已预览 0.2.1 转换草稿。请保存文件，再明确应用。' : 'Conversion draft is ready. Save the file, then explicitly Apply it.');
    } catch (error) { setMessage(error instanceof Error ? error.message : String(error), true); }
    finally { setBusy(false); }
  };
  const savePoolGraph = async (): Promise<void> => {
    if (!graph || busy) return;
    setBusy(true);
    try {
      const latest = (await getGraphPool()).graphs;
      setPoolEntries(latest);
      const bound = latest.find((entry) => entry.name === poolName);
      const name = bound ? poolName : window.prompt(lang === 'cn' ? '原文件不存在。请输入新文件名：' : 'The original file is missing. Name a new graph file:', poolName || '')?.trim();
      if (!name) return;
      if (!bound && latest.some((entry) => entry.name === name)) { setMessage(lang === 'cn' ? '该文件名已存在。请选择其他名称。' : 'That file name already exists. Choose another name.', true); return; }
      let savedEntry;
      try { savedEntry = await putGraphPoolEntry(name, bound ? poolRevision : '', graph); }
      catch (error) {
        const afterFailure = (await getGraphPool()).graphs;
        setPoolEntries(afterFailure);
        if (!bound || afterFailure.some((entry) => entry.name === name)) throw error;
        const newName = window.prompt(lang === 'cn' ? '原文件已删除。请输入新文件名：' : 'The file was deleted. Name a new graph file:', name)?.trim();
        if (!newName || afterFailure.some((entry) => entry.name === newName)) return;
        savedEntry = await putGraphPoolEntry(newName, '', graph);
      }
      setPoolRevision(savedEntry.revision);
      setPoolName(savedEntry.name); setPoolSelected(savedEntry.name);
      setRevision(savedEntry.revision); setSaved(JSON.stringify(savedEntry.graph));
      localStorage.setItem('isosui-graph-pool-last-name', savedEntry.name);
      setPoolEntries((await getGraphPool()).graphs);
      setMessage(lang === 'cn' ? `已保存图库文件“${savedEntry.name}”。` : `Saved graph file “${savedEntry.name}”.`);
    } catch (error) { setMessage(error instanceof Error ? error.message : String(error), true); }
    finally { setBusy(false); }
  };
  const newDemoGraph = async (template: 'demo' | 'package' = 'demo'): Promise<void> => {
    if (busy) return;
    if (dirty && !window.confirm(copy.discard)) return;
    setBusy(true);
    try {
      const document = await getNewNodeGraph();
      if (!nodeGraphHasEditableShape(document.graph) || document.graph.version !== '0.2.1') throw new Error('IsOS did not provide an editable 0.2.1 graph.');
      const catalog = await getNodeTypes();
      editGeneration.current += 1;
      catalogReloadRequest.current += 1;
      setGraph(template === 'package' ? packageGraphDraft(document.graph) : document.graph);
      setRawGraph(null);
      setRevision(document.revision);
      setSaved('');
      setPoolName(''); setPoolRevision('');
      setNewEmptyDraft(true);
      setTypes(catalog.types); setDataTypes(catalog.data_types || []); setAdapterCatalog(catalog);
      setReadOnlyReason(null); setGraphLoadError('');
      setDiagnostics([]); setWarnings([]); setValidatedEdges([]); setValidationStamp(null);
      setSelected([]); setSelectedEdge(''); setInspectorTarget('none'); setUndoStack([]); setRedoStack([]);
      setPositions({}); setPanelWidths({ ...DEFAULT_GRAPH_PANEL_WIDTHS }); setMirrors({}); setOptionSnapshot(null);
      setMessage(template === 'package'
        ? (lang === 'cn' ? '已载入空白软件包图草稿。添加节点后保存到新的图库文件。' : 'Empty package graph loaded as a draft. Add nodes, then save it as a new graph file.')
        : (lang === 'cn' ? '已载入当前节点演示草稿。保存后才会替换已保存的节点图。' : 'Current node demo loaded as a draft. Save it to replace the saved graph.'));
    } catch (error) { setMessage(error instanceof Error ? error.message : String(error), true); }
    finally { setBusy(false); }
  };
  const refresh = useCallback(async (confirm = false): Promise<void> => {
    if (confirm && dirty && !window.confirm(copy.discard)) return;
    const generation = editGeneration.current;
    const catalogRequestId = ++catalogReloadRequest.current;
    const statusRequestId = ++statusRequest.current;
    const readinessRequestId = ++readinessRequest.current;
    setBusy(true);
    setValidationStamp(null);
    setValidatedEdges([]);
    setEdgeInputInvalid(false);
    setDiagnostics([]);
    setWarnings([]);
    try {
      const snapshot = await getGraphCatalogSnapshot();
      const { graph: document } = snapshot;
      const catalog = combineNodeTypeCatalogs(snapshot.selected, snapshot.global);
      if (catalogRequestId !== catalogReloadRequest.current) return;
      if (generation !== editGeneration.current) { setMessage(lang === 'cn' ? '刷新期间节点图已更改，保留未保存的修改。' : 'Graph changed during refresh; unsaved edits were preserved.'); return; }
      const shaped = nodeGraphHasEditableShape(document.graph);
      if (shaped) {
        const layoutKey = graphLayoutKey(document.graph);
        if (layoutKeyRef.current !== layoutKey) {
          setPositions(parseNodePositions(localStorage.getItem(layoutKey)));
          setPanelWidths(parseGraphPanelWidths(localStorage.getItem(graphPanelWidthsKey(document.graph))));
          setMirrors(parseNodeMirrors(localStorage.getItem(graphMirrorKey(document.graph))));
          layoutKeyRef.current = layoutKey;
        }
      }
      setGraph(shaped ? document.graph : null);
      setNewEmptyDraft(false);
      setRawGraph(shaped ? null : document.graph);
      setSaved(JSON.stringify(document.graph));
      setRevision(document.revision);
      if (document.pool_name) { setPoolName(document.pool_name); setPoolSelected(document.pool_name); setPoolRevision(document.revision); }
      setTypes(catalog?.types || []);
      setDataTypes(catalog?.data_types || []); setAdapterCatalog(catalog); setOptionSnapshot(null);
      void getProcessStatus().then((status) => { if (!statusStreamReady.current && catalogRequestId === catalogReloadRequest.current && statusRequestId === statusRequest.current) setProcessStatus(status); }).catch(() => undefined);
      void getCoreReadiness().then((cores) => { if (catalogRequestId === catalogReloadRequest.current && readinessRequestId === readinessRequest.current) setReadiness(cores); }).catch(() => undefined);
      setTypeId('');
      setSelected((current) => shaped ? current.filter((path) => graphNodes(document.graph, []).some((entry) => entry.path === path)) : []);
      setDiagnostics(document.diagnostics || []);
      setWarnings(document.warnings || []);
      setReadOnlyReason(graphCatalogReadOnlyReason(snapshot));
      setGraphLoadError('');
      setUndoStack([]); setRedoStack([]); setMessage('');
    }
    catch (error) { if (catalogRequestId === catalogReloadRequest.current && generation === editGeneration.current) { const reason = error instanceof Error ? error.message : copy.disconnected; setReadOnlyReason(reason); setGraphLoadError(reason); setMessage(reason, true); } }
    finally { if (catalogRequestId === catalogReloadRequest.current) setBusy(false); }
  }, [dirty, copy.discard, copy.disconnected, lang]);
  useEffect(() => { void (async () => {
    await refresh();
    try {
      const pool = await getGraphPool();
      setPoolEntries(pool.graphs);
      const remembered = localStorage.getItem('isosui-graph-pool-last-name');
      if (remembered && !pool.graphs.some((entry) => entry.name === remembered) && !pool.graphs.length) { setPoolName(remembered); setPoolRevision(''); return; }
      const name = pool.graphs.find((entry) => entry.name === remembered)?.name || pool.graphs.find((entry) => entry.name === 'demo')?.name || pool.graphs[0]?.name;
      if (name) { setPoolSelected(name); await loadPoolGraph(name); if (remembered && remembered !== name) setFallbackNotice(lang === 'cn' ? `上次使用的图库文件“${remembered}”不存在，已打开“${name}”。` : `The last graph file “${remembered}” is missing. Opened “${name}”.`); }
    } catch { /* The active graph error is already shown by refresh. */ }
  })(); }, []);
  useEffect(() => {
    if (!visible) return;
    let active = true;
    let statusPending = false;
    let readinessPending = false;
    const unsubscribe = subscribeRuntimeStatus((snapshot) => {
      statusStreamReady.current = snapshot !== null;
      if (snapshot) { ++statusRequest.current; setProcessStatus(snapshot.process_status); }
    });
    const statusTimer = window.setInterval(() => {
      if (document.visibilityState !== 'visible' || statusPending || statusStreamReady.current) return;
      statusPending = true;
      const requestId = ++statusRequest.current;
      void getProcessStatus().then((status) => { if (active && !statusStreamReady.current && requestId === statusRequest.current) setProcessStatus(status); })
        .catch(() => undefined).finally(() => { statusPending = false; });
    }, 1500);
    const readinessTimer = window.setInterval(() => {
      if (document.visibilityState !== 'visible' || readinessPending) return;
      readinessPending = true;
      const requestId = ++readinessRequest.current;
      void getCoreReadiness().then((readiness) => { if (active && requestId === readinessRequest.current) setReadiness(readiness); })
        .catch(() => undefined).finally(() => { readinessPending = false; });
    }, 30000);
    return () => { active = false; statusStreamReady.current = false; unsubscribe(); ++statusRequest.current; ++readinessRequest.current; window.clearInterval(statusTimer); window.clearInterval(readinessTimer); };
  }, [visible]);
  useEffect(() => {
    const reloadCatalog = (): void => { setValidatedEdges([]); const requestId = ++catalogReloadRequest.current; optionRequest.current += 1; setOptionSnapshot(null); setAdapterCatalog(null); setPackageGeneration((value) => value + 1); void getGraphCatalogSnapshot().then((snapshot) => {
      if (!catalogReloadIsCurrent(requestId, catalogReloadRequest.current)) return;
      const { graph: document } = snapshot;
      const catalog = combineNodeTypeCatalogs(snapshot.selected, snapshot.global);
      setTypes(catalog.types); setDataTypes(catalog.data_types || []); setAdapterCatalog(catalog);
      const currentGraph = catalogReloadGraph(document.revision, revisionForCatalog.current, graphForCatalog.current, document.graph);
      const savedReason = graphCatalogReadOnlyReason(snapshot);
      setReadOnlyReason(savedReason || (currentGraph
        ? nodeGraphReadOnlyReason({ ...document, graph: currentGraph }, catalog.types)
        : 'Saved graph revision changed; refresh before editing.'));
    }).catch((error: unknown) => { if (catalogReloadIsCurrent(requestId, catalogReloadRequest.current)) { setReadOnlyReason('Graph catalog could not be checked.'); setMessage(error instanceof Error ? error.message : String(error)); } }); };
    window.addEventListener('isosui:packages-changed', reloadCatalog);
    window.addEventListener('focus', reloadCatalog);
    const onVisible = (): void => { if (document.visibilityState === 'visible') reloadCatalog(); };
    document.addEventListener('visibilitychange', onVisible);
    return () => { window.removeEventListener('isosui:packages-changed', reloadCatalog); window.removeEventListener('focus', reloadCatalog); document.removeEventListener('visibilitychange', onVisible); catalogReloadRequest.current += 1; };
  }, []);
  useEffect(() => {
    let active = true;
    const load = (): void => { void getInstalledPackages().then((result) => { if (active) setInstalledPackages(result.installed); }).catch((error: unknown) => { if (active) setMessage(error instanceof Error ? error.message : String(error), true); }); };
    load();
    window.addEventListener('isosui:packages-changed', load);
    return () => { active = false; window.removeEventListener('isosui:packages-changed', load); };
  }, []);
  useEffect(() => { if (!graph) return; const key = graphLayoutKey(graph); const timer = window.setTimeout(() => localStorage.setItem(key, JSON.stringify(positions)), 250); return () => window.clearTimeout(timer); }, [graph?.robot.id, positions]);
  const panelWidthsKey = graph ? graphPanelWidthsKey(graph) : '';
  useEffect(() => { if (!panelWidthsKey) return; const timer = window.setTimeout(() => localStorage.setItem(panelWidthsKey, JSON.stringify(panelWidths)), 250); return () => window.clearTimeout(timer); }, [panelWidthsKey, panelWidths]);
  useEffect(() => { if (graph) localStorage.setItem(graphMirrorKey(graph), JSON.stringify(mirrors)); }, [graph?.robot.id, mirrors]);
  useEffect(() => { const beforeUnload = (event: BeforeUnloadEvent): void => { if (dirty) event.preventDefault(); }; window.addEventListener('beforeunload', beforeUnload); return () => window.removeEventListener('beforeunload', beforeUnload); }, [dirty]);
  const edit = (change: (current: NodeGraph) => NodeGraph): void => {
    if (!graph || readOnly) return;
    const next = change(graph);
    if (next === graph || JSON.stringify(next) === JSON.stringify(graph)) return;
    const oldPaths = graphNodes(graph, []).map((entry) => entry.path);
    if (graphNodes(next, []).some((entry) => !oldPaths.includes(entry.path))) {
      setPositions((current) => preserveNodePositions(oldPaths, current));
    }
    editGeneration.current += 1; setUndoStack((stack) => [...stack.slice(-49), graph]); setRedoStack([]); setGraph(next); setValidatedEdges([]); setDiagnostics([]); setWarnings([]); setValidationStamp(null);
  };
  const undo = (): void => { if (!graph || readOnly || !undoStack.length) return; editGeneration.current += 1; setRedoStack((stack) => [...stack, graph]); setGraph(undoStack[undoStack.length - 1]); setDiagnostics([]); setWarnings([]); setValidationStamp(null); setValidatedEdges([]); setUndoStack(undoStack.slice(0, -1)); };
  const redo = (): void => { if (!graph || readOnly || !redoStack.length) return; editGeneration.current += 1; setUndoStack((stack) => [...stack, graph]); setGraph(redoStack[redoStack.length - 1]); setDiagnostics([]); setWarnings([]); setValidationStamp(null); setValidatedEdges([]); setRedoStack(redoStack.slice(0, -1)); };
  const add = (automatic = false): void => { const definition = availableTypes.find((type) => catalogTypeKey(type) === typeId); if (!definition || !graph || readOnly) return; const id = automatic || !newId.trim() ? nextNodeId(graph, definition) : newId.trim(); try { edit((current) => addNode(current, definition, id)); setSelected([nodePath(definition, id)]); setInspectorTarget('node'); setSideCollapsed(false); setNewId(''); setMessage(''); } catch (error) { setMessage(error instanceof Error ? error.message : copy.duplicate, true); } };
  const addAtCursor = (definition: NodeTypeDefinition): void => {
    if (!quickAdd || !graph || readOnly || definition.deprecated) return;
    const id = nextNodeId(graph, definition);
    try {
      edit((current) => addNode(current, definition, id));
      const path = nodePath(definition, id);
      setPositions((current) => ({ ...current, [path]: { x: quickAdd.x, y: quickAdd.y } }));
      setSelected([path]); setInspectorTarget('node'); setSideCollapsed(false); setQuickAdd(null); setMessage('');
    } catch (error) { setMessage(error instanceof Error ? error.message : copy.duplicate, true); }
  };
  const openQuickAdd = (event: React.MouseEvent<HTMLDivElement>): void => {
    event.preventDefault();
    if (!graph || readOnly || !flowInstance || !canvasRef.current) return;
    const rect = canvasRef.current.getBoundingClientRect();
    const x = event.clientX - rect.left;
    const y = event.clientY - rect.top;
    const position = flowInstance.screenToFlowPosition({ x: event.clientX, y: event.clientY });
    setEdgeMenu(null); setQuickDetail(''); setQuickSearch('');
    setQuickAdd({ left: Math.max(8, Math.min(x, rect.width - 284)), top: Math.max(8, Math.min(y, rect.height - 300)), x: position.x, y: position.y });
  };
  const openEdgeMenu = (event: React.MouseEvent, edge: Edge): void => {
    event.preventDefault(); event.stopPropagation();
    if (readOnly) return;
    const rect = canvasRef.current?.getBoundingClientRect();
    if (!rect) return;
    setQuickAdd(null);
    setEdgeMenu({ left: Math.max(8, Math.min(event.clientX - rect.left, rect.width - 180)), top: Math.max(8, Math.min(event.clientY - rect.top, rect.height - 56)), id: edge.id });
  };
  const connect = async (connection: Connection): Promise<void> => {
    if (!graph || readOnly || !connection.source || !connection.target) return;
    const generation = editGeneration.current;
    const output = connection.sourceHandle?.replace(/^out:/, '') || '';
    const input = connection.targetHandle?.replace(/^in:/, '') || '';
    try {
      const candidate = connectNodes(graph, types, connection.source, output, connection.target, input);
      const result = await validateConnection(candidate, canonicalNodePath(connection.source), output, canonicalNodePath(connection.target), input);
      if (generation !== editGeneration.current) return;
      if (!result.allowed) throw new Error(result.diagnostic?.message || 'IsOS rejected this connection.');
      if (!admittedConnection(result, connection.source, output, connection.target, input)) throw new Error('No owner-supported route delivers this connection.');
      edit(() => candidate);
      setMessage('');
    } catch (error) { if (generation === editGeneration.current) setMessage(error instanceof Error ? error.message : String(error), true); }
  };
  useEffect(() => { const onKey = (event: KeyboardEvent): void => {
    if (!visible || readOnly) return;
    const target = event.target as HTMLElement;
    if (event.key === 'Escape' && (quickAdd || edgeMenu)) { setQuickAdd(null); setEdgeMenu(null); return; }
    if (target.closest('input, textarea, select, [contenteditable="true"]')) return;
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'z') { event.preventDefault(); if (event.shiftKey) redo(); else undo(); }
    else if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'y') { event.preventDefault(); redo(); }
    else if ((event.key === 'Delete' || event.key === 'Backspace') && selected.length) { event.preventDefault(); edit((current) => removeSelectedNodes(current, selected)); setPositions((current) => Object.fromEntries(Object.entries(current).filter(([path]) => !selected.includes(path)))); setSelected([]); setInspectorTarget('none'); }
    else if (event.key.toLowerCase() === 'r' && !event.ctrlKey && !event.metaKey && !quickAdd && !edgeMenu && selected.length) { event.preventDefault(); setMirrors((current) => { const next = { ...current }; for (const path of selected) next[path] = !(next[path] ?? asyncDemoLayout(entries.map((entry) => entry.path))?.mirrors[path] ?? false); return next; }); }
  }; window.addEventListener('keydown', onKey); return () => window.removeEventListener('keydown', onKey); }, [graph, undoStack, redoStack, selected, quickAdd, edgeMenu, visible, readOnly]);
  const validate = async (): Promise<boolean> => { if (!graph || readOnly) return false; const generation = editGeneration.current; setBusy(true); try { const result = await validateNodeGraph(graph); if (!validationResponseIsCurrent(generation, editGeneration.current)) return false; setDiagnostics(result.diagnostics); setWarnings(result.warnings || []); setValidatedEdges(result.valid && Array.isArray(result.edges) ? result.edges : []); setValidationStamp(draftValidationStamp(graph, generation)); setMessage(result.valid ? (result.warnings?.length ? (lang === 'cn' ? `节点图有效，存在 ${result.warnings.length} 条警告。` : `Graph is valid with ${result.warnings.length} warning(s).`) : copy.valid) : `${result.diagnostics.length} ${copy.diagnostics.toLowerCase()}`, !result.valid); return result.valid; } catch (error) { if (validationResponseIsCurrent(generation, editGeneration.current)) { setValidationStamp(null); setWarnings([]); setMessage(error instanceof Error ? error.message : String(error), true); } return false; } finally { setBusy(false); } };
  const saveSnapshot = async (snapshot: NodeGraph): Promise<string | null> => {
    if (selectorBlocked || edgeInputInvalid) { setMessage(edgeInputInvalid ? (lang === 'cn' ? '请修复连线服务质量字段。' : 'Repair the edge QoS fields.') : (lang === 'cn' ? '请等待选项更新并修复不可用的目录选项。' : 'Wait for options to refresh and repair unavailable catalog selections.'), true); return null; }
    const generation = editGeneration.current;
    const latest = (await getGraphPool()).graphs;
    setPoolEntries(latest);
    const bound = latest.find((entry) => entry.name === poolName);
    let name = poolName;
    let fileRevision = poolRevision;
    if (!bound) {
      name = window.prompt(lang === 'cn' ? '原文件不存在。请输入新文件名：' : 'The original file is missing. Name a new graph file:', name)?.trim() || '';
      if (!name) return null;
      const existing = latest.find((entry) => entry.name === name);
      if (existing) { setMessage(lang === 'cn' ? '该文件名已存在。请选择其他名称。' : 'That file name already exists. Choose another name.', true); return null; }
      fileRevision = '';
    }
    const applyChecked = (targetName: string, targetRevision: string) => validatedApply(snapshot, targetName, targetRevision, () => validationResponseIsCurrent(generation, editGeneration.current), (checked) => {
      setDiagnostics(checked.diagnostics);
      setWarnings(checked.warnings || []);
      setValidatedEdges(checked.valid && Array.isArray(checked.edges) ? checked.edges : []);
      setValidationStamp(draftValidationStamp(snapshot, generation));
    });
    let result;
    try { result = await applyChecked(name, fileRevision); }
    catch (error) {
      const afterFailure = (await getGraphPool()).graphs;
      setPoolEntries(afterFailure);
      if (!bound || afterFailure.some((entry) => entry.name === name)) {
        const savedFile = await getGraphPoolEntry(name).catch(() => null);
        if (savedFile && JSON.stringify(savedFile.graph) === JSON.stringify(snapshot)) {
          setPoolRevision(savedFile.revision); setRevision(savedFile.revision); setSaved(JSON.stringify(savedFile.graph));
          setPoolName(savedFile.name); setPoolSelected(savedFile.name);
        }
        throw error;
      }
      const newName = window.prompt(lang === 'cn' ? '原文件已删除。请输入新文件名：' : 'The file was deleted. Name a new graph file:', name)?.trim();
      if (!newName) return null;
      if (afterFailure.some((entry) => entry.name === newName)) { setMessage(lang === 'cn' ? '该文件名已存在。请选择其他名称。' : 'That file name already exists. Choose another name.', true); return null; }
      name = newName;
      result = await applyChecked(name, '');
    }
    if (result.state === 'stale') return null;
    if (result.state === 'rejected') { setMessage(result.validation.diagnostics.map((item) => item.message).join('; ') || copy.invalid, true); setSideCollapsed(false); setDiagnosticsOpen(true); return null; }
    const document = result.document;
    catalogReloadRequest.current += 1;
    const newerDraft = editGeneration.current !== generation;
    setSaved(JSON.stringify(document.graph));
    setNewEmptyDraft(false);
    setRevision(document.revision);
    setPoolName(name); setPoolSelected(name); setPoolRevision(document.revision);
    localStorage.setItem('isosui-graph-pool-last-name', name);
    void refreshPool();
    if (!newerDraft) setGraph(document.graph);
    let selected;
    try { selected = await getGraphCatalogSnapshot(); }
    catch { setReadOnlyReason('Graph catalog could not be checked.'); setMessage('Graph catalog could not be checked.', true); return null; }
    const selectedReason = graphCatalogReadOnlyReason(selected);
    if (selected.graph.revision !== document.revision || selectedReason) {
      const reason = selectedReason || 'Saved graph revision changed; refresh before running.';
      setReadOnlyReason(reason);
      setMessage(reason, true);
      return null;
    }
    const catalog = combineNodeTypeCatalogs(selected.selected, selected.global);
    setTypes(catalog.types); setDataTypes(catalog.data_types || []); setAdapterCatalog(catalog);
    setReadOnlyReason(null);
    setMessage(newerDraft ? (lang === 'cn' ? '已保存先前版本，新的修改仍未保存。' : 'Earlier draft saved; newer edits remain unsaved.') : copy.saved);
    return newerDraft ? null : document.revision;
  };
  const apply = async (): Promise<void> => { if (!graph || readOnly || busy || selectorBlocked || edgeInputInvalid) return; setBusy(true); try { await saveSnapshot(graph); } catch (error) { setMessage(error instanceof Error ? error.message : String(error), true); } finally { setBusy(false); } };
  const running = hasActiveDeployment(processStatus);
  const primaryLabel = lang === 'cn' ? (dirty ? (running ? '保存并重启' : '保存并运行') : (running ? '重启' : '保存并运行')) : (dirty ? (running ? 'Save and Restart' : 'Save and Run') : (running ? 'Restart' : 'Save and Run'));
  const runPrimary = async (): Promise<void> => {
    if (!graph || readOnly || busy || stopping || selectorBlocked || edgeInputInvalid) return;
    if (newEmptyDraft) { await apply(); return; }
    setBusy(true);
    onOpenPanel?.('logs');
    const generation = editGeneration.current;
    const stopAtStart = deploymentStopEpoch();
    try {
      const nextRevision = await saveSnapshot(graph);
      if (!nextRevision) return;
      if (generation !== editGeneration.current || stopAtStart !== deploymentStopEpoch()) { setMessage(lang === 'cn' ? '运行前节点图或停止状态已更改，请再次检查。' : 'Graph or Stop state changed before run. Check again.', true); return; }
      const latestStatus = await getProcessStatus();
      if (generation !== editGeneration.current || stopAtStart !== deploymentStopEpoch()) { setMessage(lang === 'cn' ? '运行前节点图或停止状态已更改，请再次检查。' : 'Graph or Stop state changed before run. Check again.', true); return; }
      const operation = hasActiveDeployment(latestStatus) ? 'restart' : 'run';
      await runDeployment(operation, nextRevision);
      if (stopAtStart !== deploymentStopEpoch()) return;
      const status = await getProcessStatus();
      if (stopAtStart !== deploymentStopEpoch()) return;
      setProcessStatus(status);
      const readinessRequestId = ++readinessRequest.current;
      void getCoreReadiness().then((cores) => { if (readinessRequestId === readinessRequest.current) setReadiness(cores); })
        .catch((error: unknown) => browserLog('warn', `Core readiness refresh after ${operation} failed: ${error instanceof Error ? error.message : String(error)}`));
      setMessage(processRunFeedback(status, operation, lang));
    } catch (error) { if (stopAtStart === deploymentStopEpoch()) setMessage(error instanceof Error ? error.message : String(error), true); }
    finally { setBusy(false); }
  };
  const stop = async (): Promise<void> => {
    if (stopping) return;
    const before = processStatus;
    const request = requestRunStop(before, onStopIntent, stopDeploymentConverged);
    setStopping(true);
    setMessage(lang === 'cn' ? '正在停止…' : 'Stopping…');
    try {
      const reply = await request;
      const after = await getProcessStatus().catch(() => null);
      const feedback = stopReplyFeedback(reply, before, lang, after);
      setMessage(feedback.text, feedback.alert); onStopOutcome?.(feedback, reply.terminal?.run_id || null);
      if (after) setProcessStatus(after);
      const readinessRequestId = ++readinessRequest.current;
      void getCoreReadiness().then((cores) => { if (readinessRequestId === readinessRequest.current) setReadiness(cores); })
        .catch((error: unknown) => browserLog('warn', `Core readiness refresh after Stop failed: ${error instanceof Error ? error.message : String(error)}`));
    } catch (error) { const feedback = stopFailureFeedback(error, lang); setMessage(feedback.text, feedback.alert); onStopOutcome?.(feedback); }
    finally { setStopping(false); }
  };
  const validationCurrent = !!graph && validationDisplayIsCurrent(validationStamp, graph, editGeneration.current);
  const demoLayout = useMemo(() => asyncDemoLayout(entries.map((entry) => entry.path)), [entries]);
  const nodes = useMemo<Node<FlowData>[]>(() => entries.map((entry, index) => ({ id: entry.path, type: 'profile', position: positions[entry.path] || demoLayout?.positions[entry.path] || { x: 50 + (index % 3) * 330, y: 50 + Math.floor(index / 3) * 220 }, ...nodeSizes[entry.path], zIndex: frontNode === entry.path ? 2000 : selected.includes(entry.path) ? 1 : 0, data: { entry, error: diagnostics.some((item) => item.path.includes(entry.path)), mirrored: mirrors[entry.path] ?? demoLayout?.mirrors[entry.path] ?? false, lane: demoLayout?.lanes[entry.path], lang, dataTypes, currentRun: !dirty && !readOnly && processStatus?.state === 'running' && processStatus.run_id_state === 'known' && processStatus.active_revision === revision && processStatus.saved_revision === revision, runId: !processStatus ? undefined : processStatus.state === 'running' || processStatus.state === 'degraded' ? processStatus.run_id || null : null, activeRevision: !processStatus ? undefined : processStatus.state === 'running' || processStatus.state === 'degraded' ? processStatus.active_revision || null : null, monitorReading, onSelect: (path: string) => { setSelected([path]); setSelectedEdge(''); setInspectorTarget('node'); setSideCollapsed(false); setFrontNode(path); } }, selected: selected.includes(entry.path) })).sort((a, b) => Number(a.id === frontNode) - Number(b.id === frontNode)), [entries, positions, nodeSizes, selected, frontNode, diagnostics, mirrors, demoLayout, lang, dataTypes, dirty, readOnly, processStatus, revision, monitorReading]);
  const edges = useMemo<Edge[]>(() => entries.flatMap((entry) => Object.entries(entry.node.inputs || {}).flatMap(([input, sources]) => {
    const sourceList = inputSources(sources);
    return sourceList.map((source, inputIndex) => {
    const port = entries.find((candidate) => sameNodePath(candidate.path, source.node))?.definition?.outputs.find((item) => item.name === source.output);
    const admitted = validationCurrent ? validatedEdges.find((candidate) => candidate.source === canonicalNodePath(source.node) && candidate.output === source.output && candidate.target === entry.path && candidate.input === input) : undefined;
    const visual = interfaceVisual(port, admitted ? admitted.frequency_hz : portInput(sources).desired_rate_hz?.max, admitted ? admitted.priority : source.priority ?? 0);
    const id = `${entry.path}:${input}:${inputIndex}:${canonicalNodePath(source.node)}:${source.output}`;
    const label = port?.data_type || source.output;
    const edge = { source: canonicalNodePath(source.node), sourceHandle: `out:${source.output}`, target: entry.path, targetHandle: `in:${input}`, data: { inputIndex, inputCount: sourceList.length } };
    const destinationType = entry.definition?.inputs.find((item) => item.name === input)?.data_type;
    const mismatch = !!port?.data_type && !!destinationType && port.data_type !== destinationType;
    const issue = validationCurrent ? edgeInputDiagnostics({ ...edge, inputIndex, inputCount: edge.data.inputCount }, diagnostics)[0] : undefined;
    const reason = mismatch ? (lang === 'cn' ? '端口类型不匹配' : 'Port type mismatch') : issue?.message || '';
    return { id, ...edge, label: reason ? `${label} · ${reason}` : label, ariaLabel: `${visual.label}${reason ? ` · ${reason}` : ''}`, interactionWidth: 24, focusable: true, selected: selectedEdge === id, className: `${visual.className}${reason ? ' node-graph-edge--invalid' : ''}`, style: visual.style as React.CSSProperties, animated: false };
    });
  })), [entries, selectedEdge, diagnostics, validatedEdges, validationCurrent, lang]);
  const edgeDetail = selectedEdge ? edges.find((edge) => edge.id === selectedEdge) : undefined;
  const visibleInspectorTarget: InspectorTarget = inspectorTarget === 'package' && !selectedPackage || inspectorTarget === 'type' && !selectedLibraryType || inspectorTarget === 'node' && !chosen || inspectorTarget === 'edge' && !edgeDetail || inspectorTarget === 'multi' && selected.filter((path) => entries.some((entry) => entry.path === path)).length < 2 ? 'none' : inspectorTarget;
  useEffect(() => { if (inspectorTarget !== 'none' && visibleInspectorTarget === 'none') setInspectorTarget('none'); }, [inspectorTarget, visibleInspectorTarget]);
  const onNodesChange = (changes: NodeChange[]): void => {
    if (changes.some((change) => change.type === 'dimensions')) setNodeSizes((current) => applyNodeDimensions(current, changes));
    if (changes.some((change) => change.type === 'select')) {
      const next = applyNodeSelection(selected, changes);
      setSelected(next);
      setInspectorTarget(next.length > 1 ? 'multi' : next.length ? 'node' : 'none');
      if (changes.some((change) => change.type === 'select' && change.selected)) {
        setSelectedEdge('');
        setSideCollapsed(false);
      }
    }
    setPositions((current) => {
    const moved = changes.filter((change): change is Extract<NodeChange, { type: 'position' }> => change.type === 'position' && Boolean(change.position));
    if (!moved.length) return current;
    const next = { ...current };
    let changed = false;
    for (const change of moved) if (change.position && (current[change.id]?.x !== change.position.x || current[change.id]?.y !== change.position.y)) { next[change.id] = change.position; changed = true; }
    return changed ? next : current;
    });
  };
  const displayedPanelWidths = fitGraphPanelWidths(panelWidths, viewportWidth);
  const resizeHandle = (panel: keyof GraphPanelWidths): React.ReactElement => {
    const label = panel === 'library' ? copy.catalog : copy.inspector;
    const maximum = resizeGraphPanel(displayedPanelWidths, panel, Infinity, viewportWidth)[panel];
    return <div className={`node-graph-resize node-graph-resize--${panel}`} role="separator" tabIndex={0} aria-orientation="vertical" aria-label={`${lang === 'cn' ? '调整宽度' : 'Resize'} ${label}`} aria-controls={panel === 'library' ? 'node-graph-library' : 'node-graph-inspector'} aria-valuemin={panel === 'library' ? 225 : 270} aria-valuemax={maximum} aria-valuenow={displayedPanelWidths[panel]}
      onKeyDown={(event) => { const delta = event.key === 'ArrowRight' ? 20 : event.key === 'ArrowLeft' ? -20 : 0; if (!delta) return; event.preventDefault(); setPanelWidths((current) => resizeGraphPanel(fitGraphPanelWidths(current, viewportWidth), panel, displayedPanelWidths[panel] + (panel === 'library' ? delta : -delta), viewportWidth)); }}
      onDoubleClick={() => setPanelWidths((current) => resizeGraphPanel(fitGraphPanelWidths(current, viewportWidth), panel, DEFAULT_GRAPH_PANEL_WIDTHS[panel], viewportWidth))}
      onPointerDown={(event) => { event.preventDefault(); panelResize.current = { panel, x: event.clientX, width: displayedPanelWidths[panel] }; event.currentTarget.setPointerCapture(event.pointerId); }}
      onPointerMove={(event) => { const start = panelResize.current; if (!start || start.panel !== panel) return; const change = (event.clientX - start.x) * (panel === 'library' ? 1 : -1); setPanelWidths((current) => resizeGraphPanel(fitGraphPanelWidths(current, viewportWidth), panel, start.width + change, viewportWidth)); }}
      onPointerUp={() => { panelResize.current = null; }} onPointerCancel={() => { panelResize.current = null; }} />;
  };
  return <ReactFlowProvider><main className="node-graph-manager">
    <header><h1>{copy.title}</h1>{panelMenu}<div className="node-graph-actions" role="toolbar" aria-label={copy.title}>
      <button className="node-graph-icon-button" type="button" onClick={undo} disabled={readOnly || !undoStack.length || busy} title={`${copy.undo} (Ctrl+Z)`} aria-label={copy.undo}><span className="node-graph-history-arrow" aria-hidden="true" /></button>
      <button className="node-graph-icon-button" type="button" onClick={redo} disabled={readOnly || !redoStack.length || busy} title={`${copy.redo} (Ctrl+Shift+Z)`} aria-label={copy.redo}><span className="node-graph-history-arrow node-graph-history-arrow--redo" aria-hidden="true" /></button>
      <div className="node-graph-view"><button type="button" aria-expanded={viewOpen} onClick={() => setViewOpen((value) => !value)}>{lang === 'cn' ? '视图' : 'View'} ▾</button>{viewOpen && <div className="node-graph-view__menu" role="menu"><button type="button" role="menuitemcheckbox" aria-checked={!libraryCollapsed} onClick={() => setLibraryCollapsed((value) => !value)}>{copy.catalog}</button><button type="button" role="menuitemcheckbox" aria-checked={!sideCollapsed} onClick={() => setSideCollapsed((value) => !value)}>{copy.inspector}</button><button type="button" role="menuitemcheckbox" aria-checked={diagnosticsOpen && !sideCollapsed} onClick={() => { if (diagnosticsOpen && !sideCollapsed) setDiagnosticsOpen(false); else { setSideCollapsed(false); setDiagnosticsOpen(true); } }}>{copy.diagnostics}</button><button type="button" role="menuitem" onClick={() => onOpenPanel?.('logs')}>{lang === 'cn' ? '日志' : 'Logs'}</button><button type="button" role="menuitem" onClick={() => onOpenPage?.('eos')}>EOS</button><button type="button" role="menuitem" onClick={() => onOpenPage?.('ei')}>EI</button></div>}</div>
      <button type="button" onClick={() => { browserLog('info', `Fit graph requested; ${entries.length} nodes; viewport ${JSON.stringify(flowInstance?.getViewport())}`); setViewLost(false); void flowInstance?.fitView({ padding: 0.15, duration: 200 }); }}>{lang === 'cn' ? '显示全图' : 'Fit graph'}</button>
      <button type="button" className="node-graph-primary-action" onClick={() => void runPrimary()} disabled={readOnly || busy || !graph || selectorBlocked || edgeInputInvalid} title={lang === 'cn' ? '保存更改（如有）并启动；运行中则重启' : 'Save changes if needed, then start or restart the deployment'}>{newEmptyDraft ? copy.apply : primaryLabel}</button>
      <button type="button" onClick={() => onOpenPanel?.('logs')}>{lang === 'cn' ? '日志' : 'Logs'}</button>
      {(running || busy) && <button type="button" onClick={() => void stop()} disabled={stopping}>{lang === 'cn' ? '停止' : 'Stop'}</button>}
      <details className="node-graph-advanced" open={advancedOpen} onToggle={(event) => setAdvancedOpen(event.currentTarget.open)}><summary>{lang === 'cn' ? '高级操作' : 'Advanced'} ▾</summary><div>
        <button type="button" onClick={() => void validate()} disabled={readOnly || busy || !graph}>{copy.validate}</button><small>{lang === 'cn' ? '检查草稿，不保存' : 'Check draft without saving'}</small>
        <button type="button" onClick={() => void apply()} disabled={readOnly || busy || !graph || selectorBlocked || edgeInputInvalid}>{copy.apply}</button><small>{lang === 'cn' ? '检查并应用图库文件，同时生成核心配置' : 'Check and Apply the graph file; generate core configuration'}</small>
        <button type="button" onClick={() => void (poolName ? loadPoolGraph(poolName) : refresh(true))} disabled={busy}>{lang === 'cn' ? '重新载入当前文件' : 'Reload current file'}</button><small>{lang === 'cn' ? '丢弃修改并重新读取当前图库文件' : 'Discard edits and reread the current graph file'}</small>
        <button type="button" onClick={() => void newDemoGraph()} disabled={busy}>{lang === 'cn' ? '载入当前节点演示' : 'Load current node demo'}</button><small>{lang === 'cn' ? '载入当前演示草稿；保存后才会替换当前节点图' : 'Load the current demo as a draft; the saved graph changes only when you save it'}</small>
        <button type="button" onClick={() => void newDemoGraph('package')} disabled={busy}>{lang === 'cn' ? '新建软件包图' : 'New package graph'}</button><small>{lang === 'cn' ? '从空白草稿开始；添加节点后另存为新文件' : 'Start with an empty draft; add nodes and save as a new file'}</small>
        <label>{lang === 'cn' ? '图库文件' : 'Graph file'}<select value={poolSelected} onChange={(event) => { setPoolSelected(event.target.value); }}><option value="">—</option>{poolEntries.map((entry) => <option key={entry.name} value={entry.name}>{entry.name}</option>)}</select></label>
        <small>{poolName ? (lang === 'cn' ? `当前编辑：${poolName}${poolFileExists ? '。保存将替换同名文件。' : '。原文件不存在；保存时需要新文件名。'}` : `Editing: ${poolName}${poolFileExists ? '. Save will replace the file with the same name.' : '. The file is missing; Save will ask for a new name.'}`) : (lang === 'cn' ? '尚未绑定图库文件；保存时需要文件名。' : 'No graph file is bound; Save will ask for a name.')}</small>
        <button type="button" onClick={() => void refreshPool()} disabled={busy}>{lang === 'cn' ? '刷新图库' : 'Refresh pool'}</button>
        <button type="button" onClick={() => void loadPoolGraph()} disabled={busy || !poolSelected}>{lang === 'cn' ? '打开文件' : 'Open file'}</button>
        {graph?.version === '0.2.0' && <button type="button" onClick={() => void previewGraphConversion()} disabled={busy || !poolName}>{lang === 'cn' ? '预览转换为 0.2.1' : 'Preview conversion to 0.2.1'}</button>}
        <button type="button" onClick={() => void savePoolGraph()} disabled={busy || !graph}>{lang === 'cn' ? '保存文件' : 'Save file'}</button><small>{lang === 'cn' ? '保存文件后，应用时才更新部署配置' : 'Save the file, then Apply to update deployment configuration'}</small>
      </div></details>
      <span role="status">{dirty ? copy.dirty : copy.clean} · {copy.savedRevision}: {revision.slice(0, 8) || '—'} · {copy.activeRevision}: {processStatus?.active_revision?.slice(0, 8) || '—'}</span>
    </div><div className="node-graph-core-statuses" aria-label={lang === 'cn' ? '归属方状态' : 'Owner status'}><CoreStatusBadge name="EI" core={readiness?.cores.find((core) => core.core === 'ei')} lang={lang} /><CoreStatusBadge name="EOS" core={readiness?.cores.find((core) => core.core === 'eos')} lang={lang} /></div></header>
    {!onNoticesChange && <>{catalogNotice && <p className="node-graph-error" role="alert">{catalogNotice}</p>}{fallbackNotice && <p className="node-graph-note" role="status">{fallbackNotice}</p>}{readOnlyReason && <p className="node-graph-error" role="alert">{lang === 'cn' ? '只读节点图：' : 'Read-only graph: '}{readOnlyReason}</p>}{viewLost && <p className="node-graph-note" role="status">{lang === 'cn' ? '节点可能在视野之外。使用“显示全图”恢复。' : 'Nodes may be outside the view. Use Fit graph to recover.'}</p>}{processStatus?.last_error && <p className="node-graph-error" role="alert">{processStatus.last_error}</p>}</>}
    {startupPhaseText(processStatus, lang, busy) && <p className="node-graph-note" role="status">{startupPhaseText(processStatus, lang, busy)} · <button type="button" onClick={() => onOpenPanel?.('logs')}>{lang === 'cn' ? '查看日志' : 'View Logs'}</button></p>}
    {graphLoadError && <p className="node-graph-error" role="alert">{lang === 'cn' ? '无法打开已保存的节点图。' : 'The saved graph cannot be opened.'} <button type="button" onClick={() => void newDemoGraph()} disabled={busy}>{lang === 'cn' ? '载入当前节点演示' : 'Load current node demo'}</button></p>}
    {graph && readOnly && (graphPackagePins(graph).length > 0 || readiness?.cores.some((entry) => !entry.ready && entry.reason)) && <GraphDependencyRepair graph={graph} installed={installedPackages} readiness={readiness} lang={lang} onInstalled={() => { refreshLibraries(); void loadPoolGraph(poolName); }} onRetry={() => void loadPoolGraph(poolName)} />}
    <div className={`node-graph-body${libraryCollapsed ? ' node-graph-body--library-collapsed' : ''}${sideCollapsed ? ' node-graph-body--side-collapsed' : ''}`} style={{ '--library-width': `${displayedPanelWidths.library}px`, '--side-width': `${displayedPanelWidths.side}px` } as React.CSSProperties}><aside id="node-graph-library" className="node-graph-library" hidden={libraryCollapsed}><div className="node-graph-library__rail" role="group" aria-label={lang === 'cn' ? '图库视图' : 'Library view'}><button type="button" aria-pressed={libraryView === 'nodes'} aria-label={copy.catalog} title={copy.catalog} onClick={() => setLibraryView('nodes')}><span aria-hidden="true">▤</span></button><button type="button" aria-pressed={libraryView === 'packages'} aria-label={lang === 'cn' ? '软件包库' : 'Package library'} title={lang === 'cn' ? '软件包库' : 'Package library'} onClick={() => setLibraryView('packages')}><span aria-hidden="true">▣</span></button></div><h2>{libraryView === 'nodes' ? copy.catalog : (lang === 'cn' ? '软件包库' : 'Package library')}</h2>{libraryView === 'nodes' ? <CatalogBrowser types={availableTypes} value={typeId} lang={lang} onChange={selectLibraryType} /> : <PackageLibrary installed={installedPackages} types={availableTypes} selectedPin={selectedPackagePin} selectedType={typeId} lang={lang} onSelectPackage={(pin) => { if (pin === selectedPackagePin) { setInspectorTarget('package'); setSideCollapsed(false); return true; } if (packageConfigDirty && !window.confirm(lang === 'cn' ? '软件包配置有未保存的修改。要切换软件包吗？' : 'Package configuration has unsaved changes. Switch packages?')) return false; setPackageConfigDirty(false); setSelectedPackagePin(pin); setInspectorTarget('package'); setSideCollapsed(false); return true; }} onSelectType={selectLibraryType} onRefresh={() => void refreshLibraries()} />}<label>{copy.id}<input value={newId} placeholder={graph && selectedLibraryType ? nextNodeId(graph, selectedLibraryType) : ''} onChange={(event) => setNewId(event.target.value.toLowerCase())} /></label><button type="button" onClick={() => add()} disabled={readOnly || !graph || !selectedLibraryType}>{copy.add}</button>{libraryView === 'nodes' && <><p>{copy.port}</p><p>{copy.hint}</p><div className="node-graph-legend" aria-label={lang === 'cn' ? '端口颜色' : 'Port colors'}><h3>{lang === 'cn' ? '端口颜色：准确数据类型' : 'Port color: exact data type'}</h3>{(dataTypes.length ? dataTypes.slice(0, 5).map((item) => item.id) : ['camera_frame', 'vision_frame', 'base_observation']).map((id) => <span key={id} style={{ '--port': dataTypeColor(id) } as React.CSSProperties}><i aria-hidden="true" />{id}</span>)}</div><div className="node-graph-interface-legend" aria-label={lang === 'cn' ? '接口视觉规则' : 'Interface visual rules'}><strong>{lang === 'cn' ? '接口视觉规则' : 'Interface visual rules'}</strong><span>{lang === 'cn' ? '端口形状表示通道：进程内圆形 / Unix 方形 / HTTP 盾形 / WebSocket 菱形 / ROS 2 六边形' : 'Port shape shows channel: in-process circle / Unix square / HTTP shield / WebSocket diamond / ROS 2 hexagon'}</span><span>{lang === 'cn' ? '连线图案表示连线上限：<1 Hz 稀疏点；1–<10 Hz 点线；10–<30 Hz 长划线；≥30 Hz 实线；无限制为长短交替线' : 'Line pattern shows edge cap: <1 Hz sparse dots; 1–<10 Hz dotted; 10–<30 Hz long dashes; ≥30 Hz solid; unlimited alternating dash'}</span><span>{lang === 'cn' ? '连线颜色表示优先级：红色较高，蓝色较低；具体 nice 值见连线详情' : 'Line color shows priority: red is higher, blue is lower; inspect the line for its exact nice value'}</span></div></>}{graph && <><h2>{copy.robot}</h2><label>{copy.id}<input value={graph.robot.id} onChange={(event) => edit((current) => ({ ...current, robot: { ...current.robot, id: event.target.value } }))} /></label><label>{copy.name}<input value={graph.robot.name} onChange={(event) => edit((current) => ({ ...current, robot: { ...current.robot, name: event.target.value } }))} /></label></>}</aside>
      <div className="node-graph-canvas" ref={canvasRef} onContextMenu={openQuickAdd} onPointerDownCapture={(event) => { if (event.button === 0 && !(event.target as HTMLElement).closest('.node-graph-edge-menu, .node-graph-quick-add')) { setQuickAdd(null); setEdgeMenu(null); } activeGesture.current = true; }} onPointerUpCapture={() => { activeGesture.current = false; }} onPointerCancelCapture={() => { activeGesture.current = false; }} onMouseMove={revealOverview} onTouchMove={revealOverview}>{graph ? <ReactFlow nodes={nodes} edges={edges} nodeTypes={nodeTypes} nodesConnectable={!readOnly} elementsSelectable elevateNodesOnSelect={false} onInit={(instance) => { setFlowInstance(instance); browserLog('debug', 'Graph canvas initialized'); }} onEdgeContextMenu={openEdgeMenu} onEdgeClick={(_, edge) => { setEdgeInputInvalid(false); setSelectedEdge(edge.id); setSelected([]); setInspectorTarget('edge'); setSideCollapsed(false); setQuickAdd(null); }} onMove={revealOverview} onNodesChange={onNodesChange} onConnect={connect} onNodeClick={(_, node) => { setFrontNode(node.id); setSelected([node.id]); setInspectorTarget('node'); setSideCollapsed(false); setQuickAdd(null); setSelectedEdge(''); setEdgeInputInvalid(false); }} onNodeDragStart={(_, node) => setFrontNode(node.id)} onSelectionStart={() => { activeGesture.current = true; }} onSelectionEnd={() => { activeGesture.current = false; }} selectionOnDrag selectionMode={SelectionMode.Partial} panOnDrag={[1]} deleteKeyCode={null} onPaneClick={() => { setSelected([]); setQuickAdd(null); setEdgeMenu(null); setSelectedEdge(''); setInspectorTarget('none'); setEdgeInputInvalid(false); }} onDoubleClick={(event) => { if ((event.target as HTMLElement).classList.contains('react-flow__pane')) add(true); }} minZoom={0.2} maxZoom={2}><Background color="var(--border)" /><MiniMap className={showOverview ? 'node-graph-minimap--visible' : 'node-graph-minimap--hidden'} pannable zoomable /></ReactFlow> : rawGraph ? <pre className="node-graph-raw" aria-label={lang === 'cn' ? '原始节点图' : 'Raw node graph'}>{JSON.stringify(rawGraph, null, 2)}</pre> : <div className="node-graph-empty"><p>{copy.disconnected}</p><button type="button" onClick={() => void refresh()}>{copy.connect}</button></div>}{graph && !entries.length && <p className="node-graph-empty">{copy.noNodes}</p>}{edgeMenu && <div className="node-graph-edge-menu" role="menu" style={{ left: edgeMenu.left, top: edgeMenu.top }}><button type="button" role="menuitem" onClick={() => { const edge = edges.find((item) => item.id === edgeMenu.id); if (edge) edit((current) => disconnectInput(current, edge.target, String(edge.targetHandle).replace(/^in:/, ''), edge.source, String(edge.sourceHandle).replace(/^out:/, ''))); setEdgeMenu(null); }}>{copy.disconnect}</button></div>}{quickAdd && <div className="node-graph-quick-add" role="dialog" aria-label={copy.add} style={{ left: quickAdd.left, top: quickAdd.top }} onContextMenu={(event) => event.stopPropagation()}><input ref={quickSearchRef} type="search" aria-label={copy.search} placeholder={copy.search} value={quickSearch} onChange={(event) => setQuickSearch(event.target.value)} /><div className="node-graph-quick-add__list">{quickTypes.map((type) => <button type="button" key={catalogTypeKey(type)} onMouseEnter={() => setQuickDetail(catalogTypeKey(type))} onFocus={() => setQuickDetail(catalogTypeKey(type))} onClick={() => addAtCursor(type)}><strong>{type.group}</strong><small>{type.core} / {type.layer} · {type.id} · {type.package ? `${type.package.id}@${type.package.version}` : (lang === 'cn' ? '内置' : 'Built-in')}</small></button>)}</div>{quickDetail && <div className="node-graph-quick-add__detail" role="note">{(() => { const type = quickTypes.find((item) => catalogTypeKey(item) === quickDetail); return type ? <><strong>{type.id}</strong><p>{type.core} / {type.layer} / {type.group} · {type.lifecycle} · {type.package ? `${type.package.id}@${type.package.version}` : (lang === 'cn' ? '内置' : 'Built-in')}</p><p>{type.capabilities.join(', ') || '—'}</p><small>{type.inputs.length} {lang === 'cn' ? '输入' : 'inputs'} · {type.outputs.length} {lang === 'cn' ? '输出' : 'outputs'} · {type.settings.length} {lang === 'cn' ? '设置' : 'settings'}</small></> : null; })()}</div>}</div>}

      </div>
      <aside id="node-graph-inspector" className="node-graph-side" hidden={sideCollapsed}>{selectedPackage && <div hidden={visibleInspectorTarget !== 'package'}>{/* Keep the configuration draft mounted across inspector selections. */}<section className="node-package-details" style={{ '--package-accent': packageAccent(selectedPackagePin) } as React.CSSProperties}><h2><span className="node-library-package-dot" aria-hidden="true" />{selectedPackagePin}</h2><dl><dt>{lang === 'cn' ? '来源' : 'Source'}</dt><dd>{selectedPackage.local_link ? (lang === 'cn' ? '本机链接' : 'Local link') : (lang === 'cn' ? '已安装归档' : 'Installed archive')}</dd><dt>{lang === 'cn' ? '源提交' : 'Source commit'}</dt><dd><code>{selectedPackage.source_commit || '—'}</code></dd><dt>SHA-256</dt><dd><code>{selectedPackage.asset_sha256 || '—'}</code></dd></dl>{selectedPackage.configurable ? <PackageConfigPanel key={selectedPackagePin} id={selectedPackage.id} version={selectedPackage.version} lang={lang} onDirtyChange={setPackageConfigDirty} onSaved={() => window.dispatchEvent(new Event('isosui:packages-changed'))} /> : <p>{lang === 'cn' ? '此软件包没有配置描述。' : 'This package has no configuration descriptor.'}</p>}</section></div>}{visibleInspectorTarget === 'edge' && edgeDetail ? <EdgeIdlPanel key={edgeDetail.id} edge={edgeDetail} entries={entries} dataTypes={dataTypes} diagnostics={diagnostics} validated={validationCurrent} processStatus={processStatus} lang={lang} /> : visibleInspectorTarget === 'node' && chosen ? <><fieldset disabled={readOnly} className="node-graph-readonly-fieldset"><NodeInspector key={chosen.path} entry={chosen} entries={entries} dataTypes={dataTypes} lang={lang} options={displayOptions} optionsCurrent={!!nodeOptions && optionErrorDraft !== draftKey} optionsFailed={!optionRefreshing && (!nodeOptions || optionErrorDraft === draftKey)} adapterPreviewAvailable={!!previewOptions} runId={processStatus?.run_id || null} activeRevision={processStatus?.active_revision || null} currentRun={!dirty && !readOnly && processStatus?.state === 'running' && processStatus.run_id_state === 'known' && processStatus.active_revision === revision && processStatus.saved_revision === revision} onBind={(target) => { try { edit((current) => bindPrimitiveTarget(current, types, chosen.path, target)); setMessage(''); } catch (error) { setMessage(error instanceof Error ? error.message : String(error), true); } }} onChange={(change) => edit((current) => updateNode(current, chosen.path, change))} onRename={(id) => { try { edit((current) => renameNode(current, chosen.path, id)); setSelected([`${chosen.path.slice(0, chosen.path.lastIndexOf('/') + 1)}${id}`]); setMessage(''); } catch { setMessage(copy.duplicate, true); } }} onRemove={() => { edit((current) => removeNode(current, chosen.path)); setPositions((current) => Object.fromEntries(Object.entries(current).filter(([path]) => path !== chosen.path))); setSelected([]); setInspectorTarget('none'); }} /><PortRateEditor entry={chosen} entries={entries} lang={lang} onChange={(input, rate) => edit((current) => updateDesiredRate(current, chosen.path, input, rate))} /></fieldset><h3>{copy.target}</h3><ul>{Object.entries(chosen.node.inputs || {}).flatMap(([input, sources]) => inputSources(sources).map((source) => <li key={`${input}:${source.node}:${source.output}`}>{input} ← {canonicalNodePath(source.node)}.{source.output}<button type="button" aria-label={`${copy.remove} ${input}`} disabled={readOnly} onClick={() => edit((current) => disconnectInput(current, chosen.path, input, source.node, source.output))}>×</button></li>))}</ul></> : visibleInspectorTarget === 'multi' && selected.length > 1 ? <div className="node-graph-multi-selection"><p><strong>{selected.length}</strong> {copy.selectedCount}</p><button type="button" disabled={readOnly} onClick={() => { edit((current) => removeSelectedNodes(current, selected)); setPositions((current) => Object.fromEntries(Object.entries(current).filter(([path]) => !selected.includes(path)))); setSelected([]); setInspectorTarget('none'); }}>{copy.remove}</button></div> : visibleInspectorTarget === 'type' && selectedLibraryType ? <div className="node-library-type-details"><h2>{selectedLibraryType.id}</h2><p>{selectedLibraryType.core} / {selectedLibraryType.layer} / {selectedLibraryType.group}</p><p>{packagePin(selectedLibraryType) || (lang === 'cn' ? '内置' : 'Built-in')}</p><p>{selectedLibraryType.description || '—'}</p></div> : visibleInspectorTarget === 'package' ? null : <p>{copy.noSelection}</p>}
        <details className="node-graph-panel" open={diagnosticsOpen} onToggle={(event) => setDiagnosticsOpen(event.currentTarget.open)}>
          <summary><strong>{copy.diagnostics}</strong><span>{diagnostics.length + warnings.length || '✓'}</span></summary>
          {validationCurrent || warnings.length ? diagnostics.length || warnings.length ? <ul>{[...diagnostics, ...warnings].map((item, index) => <li key={index}><button className="node-graph-diagnostic" type="button" onClick={() => { setSelected([entries.find((entry) => item.path.includes(entry.path))?.path || ''].filter(Boolean)); setInspectorTarget('node'); setSideCollapsed(false); }}><strong>{index < diagnostics.length ? (lang === 'cn' ? '错误' : 'Error') : (lang === 'cn' ? '警告' : 'Warning')}</strong><code>{item.path}</code><span>{item.message}</span></button></li>)}</ul> : <p>{copy.valid}</p> : <p>{lang === 'cn' ? '当前草稿尚未检查。' : 'Current draft has not been checked.'}</p>}
        </details>
        <details className="node-graph-panel">
          <summary><strong>{copy.process}</strong><span>{processStatus?.processes.length ?? '—'}</span></summary>
          {processStatus ? <ul>{processStatus.processes.map((process) => <li key={process.id}><strong>{process.id}</strong><span>{process.state || 'running'} · PID {process.pid ?? '—'} · {process.restarts} restarts</span></li>)}</ul> : <p>{copy.noStatus}</p>}
        </details></aside>
      {!libraryCollapsed && viewportWidth > 700 && resizeHandle('library')}
      {!sideCollapsed && viewportWidth > 700 && resizeHandle('side')}
    </div>
  </main></ReactFlowProvider>;
}
