import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import ReactFlow, { Background, Handle, MiniMap, Position, ReactFlowProvider, SelectionMode, type Connection, type Edge, type Node, type NodeChange, type NodeProps, type ReactFlowInstance } from 'reactflow';
import 'reactflow/dist/style.css';
import { catalogTypeKey, combineNodeTypeCatalogs, type NodeOptions, getGraphCatalogSnapshot, getNewNodeGraph, getNodeTypes, getGraphPool, getGraphPoolEntry, putGraphPoolEntry, getNodeGraph, getNodeSets, getProcessStatus, getCoreReadiness, getNodeOptions, graphCatalogReadOnlyReason, nodeGraphHasEditableShape, nodeGraphReadOnlyReason, previewNodeSet, validateConnection, validateNodeGraph, type CoreReadiness, type DataTypeDefinition, type GraphPoolEntry, type NodeGraph, type NodeGraphDiagnostic, type NodeInstance, type NodeSetDefinition, type NodeSetPreview, type NodeTypeDefinition, type ProcessStatus, type ValidatedEdge } from '@/services/isos/node-graph';
import type { Language } from '@/types';
import { hasActiveDeployment } from './deployment-phase';
import { addNode, addableTypes, admittedConnection, bindPrimitiveTarget, canonicalNodePath, connectNodes, disconnectInput, graphNodes, inputSources, nextNodeId, nodePath, removeNode, removedNodePaths, renameNode, sameNodePath, updateNode, type GraphNode } from './node-graph-edits';
import { interfaceVisual, nodeVisualStyle, portVisual, dataTypeColor } from './node-visuals';
import { applyNodeSelection, asyncDemoLayout, graphLayoutKey, graphMirrorKey, parseNodeMirrors, parseNodePositions, removeSelectedNodes, type NodeMirrors, type NodePositions } from './graph-canvas-state';
import { GraphCoreWindow, readCoreWindow, type CoreWindowState } from './GraphCoreWindow';
import { edgeContracts, edgeInputDiagnostics } from './edge-idl';
import { validatedApply } from './validated-apply';
import { EiWorkbench } from './EiWorkbench';
import { clampInfoCard } from './node-info-card';
import { processRunFeedback } from './process-run-feedback';
import { stopReplyFeedback, stopFailureFeedback } from './stop-feedback';
import { requestRunStop } from './run-stop-fence';
import { deploymentStopEpoch, runDeployment, stopDeploymentConverged } from './deployment-control';
import { browserLog } from '@/services/isos/browser-logs';
import { publishUiMessage } from '@/services/isos/ui-messages';
import { getButtonActivationResult, newButtonEventId, sendButtonActivation, type ButtonActivationResult } from '@/services/isos/node-events';
import { IsosApiError } from '@/services/isos/api-client';
import { draftValidationStamp, validationDisplayIsCurrent, validationResponseIsCurrent, type DraftValidationStamp } from './draft-validation';
import { CatalogSelector, settingOptionGroup } from './CatalogSelector';
import { CatalogBrowser } from './CatalogBrowser';
import { ArmMonitorView, useArmMonitor, type ArmMonitorReading } from './ArmMonitor';
import { DEFAULT_MONITOR_REFRESH_HZ, monitorRefreshHz } from '@/services/isos/arm-monitor';
import { catalogReloadGraph, catalogReloadIsCurrent, catalogSelectionBlocks, optionResponseIsCurrent, type OptionSnapshot } from './catalog-options';
import './node-graph-manager.scss';

interface Props { lang: Language; visible?: boolean; onDirtyChange?: (dirty: boolean) => void; onOpenPanel?: (name: 'control' | 'logs') => void; onStopIntent?: (runId: string | null) => void; onStopOutcome?: (feedback: { text: string; alert: boolean }, terminalRun?: string | null) => void }
interface FlowData { entry: GraphNode; error: boolean; mirrored: boolean; lane?: 'forward' | 'return'; lang: Language; dataTypes: DataTypeDefinition[]; options?: NodeOptions | null; readOnly?: boolean; currentRun?: boolean; runId?: string | null; activeRevision?: string | null; monitorReading?: ArmMonitorReading; onSettingsChange?: (path: string, settings: Record<string, unknown>) => void; onNodeChange?: (path: string, change: Partial<NodeInstance>) => void }
const COPY = {
  en: { nodeSets: 'Package node sets', previewRemoves: 'Package preview would remove existing nodes. Edit the graph explicitly before adding this set.', noNodeSets: 'No installed package offers a node set.', chooseSet: 'Choose a node set', previewSet: 'Preview nodes', addSet: 'Add previewed nodes', previewCount: 'nodes will be added', previewStale: 'The graph changed. Preview the set again.', title: 'Node graph', connect: 'Connect to local IsOS', add: 'Add node', refresh: 'Refresh', validate: 'Check graph', apply: 'Apply graph', saved: 'Graph file applied and core configuration generated. Restart to use this revision.', valid: 'Graph is valid.', dirty: 'Unsaved changes', clean: 'Saved', catalog: 'Node library', inspector: 'Node configuration', diagnostics: 'Diagnostics', noSelection: 'Select a node to edit it.', selectedCount: 'nodes selected', noNodes: 'Add a registered node type to start.', id: 'Node ID', label: 'Label', description: 'Description', settings: 'Settings', gui: 'Show MuJoCo window when EOS starts this backend', details: 'Hardware and advanced fields (JSON)', applyDetails: 'Apply fields', remove: 'Remove node', capabilities: 'Capabilities', source: 'Source', target: 'Target', invalid: 'Invalid JSON object.', duplicate: 'Node ID already exists or is invalid.', port: 'Connect matching typed ports by dragging from an output to an input.', robot: 'Robot', name: 'Name', lifecycle: 'Lifecycle', type: 'Type', disconnected: 'IsOS node graph API is unavailable.', stale: 'Graph changed on the server. Refresh and retry.', discard: 'Discard unsaved changes?', choose: 'Choose a type', process: 'Process status', noStatus: 'Process status is not available from this IsOS build.', search: 'Search types', undo: 'Undo', redo: 'Redo', disconnect: 'Disconnect line', hint: 'Left drag selects nodes; middle drag pans. Right-click to add or disconnect. Delete removes selected nodes.', savedRevision: 'Saved', activeRevision: 'Active' },
  cn: { nodeSets: '软件包节点集', previewRemoves: '软件包预览会删除现有节点。请先明确编辑节点图，再添加此节点集。', noNodeSets: '已安装的软件包没有提供节点集。', chooseSet: '选择节点集', previewSet: '预览节点', addSet: '添加已预览节点', previewCount: '个节点将被添加', previewStale: '节点图已更改，请重新预览。', title: '节点图', connect: '连接本机 IsOS', add: '添加节点', refresh: '刷新', validate: '检查节点图', apply: '应用节点图', saved: '图库文件已应用，核心配置已生成。重启后使用此修订。', valid: '节点图有效。', dirty: '未保存更改', clean: '已保存', catalog: '节点库', inspector: '节点配置', diagnostics: '诊断', noSelection: '选择节点后编辑配置。', selectedCount: '个节点已选中', noNodes: '添加已注册的节点类型以开始。', id: '节点 ID', label: '标签', description: '描述', settings: '设置', gui: 'EOS 启动此后端时显示 MuJoCo 窗口', details: '硬件与高级字段（JSON）', applyDetails: '应用字段', remove: '删除节点', capabilities: '能力', source: '来源', target: '目标', invalid: 'JSON 对象无效。', duplicate: '节点 ID 已存在或无效。', port: '从输出端口拖到类型相同的输入端口以连接。', robot: '机器人', name: '名称', lifecycle: '生命周期', type: '类型', disconnected: 'IsOS 节点图 API 不可用。', stale: '服务器上的图已更改。刷新后重试。', discard: '放弃未保存的更改？', choose: '选择类型', process: '进程状态', noStatus: '当前 IsOS 版本未提供进程状态。', search: '搜索类型', undo: '撤销', redo: '重做', disconnect: '断开连线', hint: '左键拖动框选节点，中键拖动平移。右键添加节点或断开连线。Delete 删除选中节点。', savedRevision: '已保存', activeRevision: '运行中' },
} as const;

export function FlowNode({ data, selected }: NodeProps<FlowData>): React.ReactElement {
  const { entry } = data;
  const [open, setOpen] = useState(false);
  const [configureOpen, setConfigureOpen] = useState(false);
  const [pendingEvent, setPendingEvent] = useState<string | null>(null);
  const [lastEventId, setLastEventId] = useState<string | null>(null);
  const [eventResult, setEventResult] = useState<ButtonActivationResult | null>(null);
  const [eventError, setEventError] = useState('');
  const [eventUncertain, setEventUncertain] = useState(false);
  const [submittedFence, setSubmittedFence] = useState<string | null>(null);
  const eventPollAbort = useRef<AbortController | null>(null);
  const [location, setLocation] = useState({ left: 8, top: 8 });
  const hideTimer = useRef<number | null>(null);
  const hoverTimer = useRef<number | null>(null);
  const pointerFocus = useRef(false);
  const cardRef = useRef<HTMLElement | null>(null);
  const configCloseRef = useRef<HTMLButtonElement | null>(null);
  const configureButtonRef = useRef<HTMLButtonElement | null>(null);
  const currentFence = data.currentRun && data.runId && data.activeRevision ? `${data.runId}:${data.activeRevision}` : null;
  const currentFenceRef = useRef(currentFence);
  currentFenceRef.current = currentFence;
  useEffect(() => { if (configureOpen) configCloseRef.current?.focus(); }, [configureOpen]);
  const closeConfigure = (): void => { setConfigureOpen(false); configureButtonRef.current?.focus(); };
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
  return <div className={`node-graph-card${selected ? ' node-graph-card--selected' : ''}${data.error ? ' node-graph-card--error' : ''}${data.mirrored ? ' node-graph-card--mirrored' : ''}`} style={nodeVisualStyle(entry.definition) as React.CSSProperties} tabIndex={0} aria-label={`${entry.node.label || entry.node.id}: ${entry.node.node_type}`} onMouseEnter={(event) => showAfterPause(event.clientX, event.clientY)} onMouseMove={(event) => showAfterPause(event.clientX, event.clientY)} onMouseLeave={hideSoon} onPointerDown={() => { pointerFocus.current = true; cancelHover(); setOpen(false); }} onPointerUp={() => { pointerFocus.current = false; }} onPointerCancel={() => { pointerFocus.current = false; }} onFocus={(event) => { if (pointerFocus.current || event.target !== event.currentTarget) return; const bounds = event.currentTarget.getBoundingClientRect(); show(bounds.right, bounds.top); }} onBlur={(event) => { if (!cardRef.current?.contains(event.relatedTarget as globalThis.Node)) hideSoon(); }} onKeyDown={(event) => { if (event.key === 'Escape') { setOpen(false); event.stopPropagation(); } }}>
    <div className="node-graph-card__heading"><small>{entry.path.split('/')[1]} / {entry.path.split('/')[2]}</small><span>{entry.node.node_type.split('@').pop() || 'v0.2'}</span></div>
    {data.lane && <small className="node-graph-card__lane">{data.lane === 'forward' ? (data.lang === 'cn' ? '前向 · 异步命令 →' : 'Forward · async command →') : (data.lang === 'cn' ? '← 返回 · 最新状态' : '← Return · latest state')}</small>}<strong>{entry.node.label || entry.node.id}</strong><button type="button" className="node-graph-card__info nodrag nowheel" aria-label={`${labels.info}: ${entry.node.label || entry.node.id}`} aria-expanded={open} onPointerDown={(event) => event.stopPropagation()} onClick={(event) => { event.stopPropagation(); const bounds = event.currentTarget.getBoundingClientRect(); show(bounds.right, bounds.top); }}>ⓘ</button>
    <div className="node-graph-card__actions nodrag nowheel"><button ref={configureButtonRef} type="button" aria-expanded={configureOpen} aria-label={`${data.lang === 'cn' ? '配置' : 'Configure'}: ${entry.node.label || entry.node.id}`} onPointerDown={(event) => event.stopPropagation()} onClick={(event) => { event.stopPropagation(); if (configureOpen) { closeConfigure(); return; } const bounds = event.currentTarget.getBoundingClientRect(); place(bounds.right, bounds.top); setOpen(false); setConfigureOpen(true); }}>{data.lang === 'cn' ? '配置' : 'Configure'}</button>{buttonNode && <button type="button" disabled={!data.currentRun || !!pendingEvent} onPointerDown={(event) => event.stopPropagation()} onClick={(event) => { event.stopPropagation(); void activate(); }}>{pendingEvent ? (data.lang === 'cn' ? '发送中…' : 'Sending…') : (data.lang === 'cn' ? '触发' : 'Activate')}</button>}</div>
    {buttonNode && (eventResult || eventError) && <small className="node-graph-card__result" role={eventError || eventUncertain || submittedFence !== currentFence ? 'alert' : 'status'}>{lastEventId && <code>{lastEventId}</code>} · {submittedFence !== currentFence ? (data.lang === 'cn' ? '旧运行的结果；当前状态已改变' : 'Prior run result; current state changed') : eventError ? `${eventUncertain ? (data.lang === 'cn' ? '结果不确定；请勿自动重试' : 'Outcome uncertain; do not retry automatically') : ''} · ${eventError}` : eventResult ? `${eventResult.status}${eventResult.joint_positions_rad ? ` · ${eventResult.joint_positions_rad.join(', ')} rad` : ''}${eventUncertain ? (data.lang === 'cn' ? ' · 结果不确定；请勿自动重试' : ' · Outcome uncertain; do not retry automatically') : ''}` : ''}</small>}
    {entry.node.node_type === 'isos.ui.monitor.arm@v0.2' && <ArmMonitorView lang={data.lang} reading={data.monitorReading || { snapshot: null, receivedAt: null, error: '' }} expectedRunId={data.runId} expectedRevision={data.activeRevision} compact />}
    <div className="node-graph-card__badges"><span>{entry.definition?.core || 'unknown'}</span><span>{entry.definition?.kind || 'unknown'}</span>{entry.node.package && <span>{entry.node.package.id}@{entry.node.package.version}</span>}</div>
    <div className="node-graph-card__ports">{!entry.definition && <span className="node-graph-card__no-ports" role="alert">{data.lang === 'cn' ? '目录类型不可用；无法显示端口' : 'Catalog type unavailable; ports cannot be shown'}</span>}{entry.definition && !entry.definition.inputs.length && !entry.definition.outputs.length && <span className="node-graph-card__no-ports">{entry.node.node_type.endsWith('.health@v0.2') ? (data.lang === 'cn' ? '无图端口 · 归属方健康' : 'No graph ports · owner health') : (data.lang === 'cn' ? '无图端口' : 'No graph ports')}</span>}<div>{entry.definition?.inputs.map((port) => <div className={`node-graph-card__port ${portVisual(port).className}`} style={portVisual(port).style as React.CSSProperties} key={port.name}><Handle type="target" position={data.mirrored ? Position.Right : Position.Left} id={`in:${port.name}`} title={`${port.name}: ${port.data_type} · ${port.transport || 'in_process'} · ${port.delivery_mode || 'snapshot'}`} /><span>{port.name}{port.required ? ' *' : ''}</span><small>{port.data_type}</small></div>)}</div><div>{entry.definition?.outputs.map((port) => <div className={`node-graph-card__port node-graph-card__port--output ${portVisual(port).className}`} style={portVisual(port).style as React.CSSProperties} key={port.name}><span>{port.name}</span><small>{port.data_type}</small><Handle type="source" position={data.mirrored ? Position.Left : Position.Right} id={`out:${port.name}`} title={`${port.name}: ${port.data_type} · ${port.transport || 'in_process'} · ${port.delivery_mode || 'snapshot'}`} /></div>)}</div></div>
    {open && createPortal(<aside ref={cardRef} className="node-graph-info-card nodrag nowheel" role="note" aria-label={`${labels.info}: ${entry.node.label || entry.node.id}`} style={location} onMouseEnter={(event) => { event.stopPropagation(); if (hideTimer.current !== null) window.clearTimeout(hideTimer.current); }} onMouseLeave={(event) => { event.stopPropagation(); hideSoon(); }} onPointerDown={(event) => event.stopPropagation()} onKeyDown={(event) => { if (event.key === 'Escape') { setOpen(false); event.stopPropagation(); } }}><button type="button" onClick={() => setOpen(false)} aria-label={labels.close}>×</button><h3>{entry.node.label || entry.node.id}</h3><code>{entry.node.node_type}</code><p><strong>{labels.type}:</strong> {entry.definition?.description || labels.missing}</p><p><strong>{labels.instance}:</strong> {entry.node.description || '—'}</p>{([['input', entry.definition?.inputs], ['output', entry.definition?.outputs]] as const).map(([direction, ports]) => <section key={direction}><h4>{labels[direction]}</h4><ul>{ports?.map((port) => { const payload = data.dataTypes.find((item) => item.id === port.data_type); const schema = payload?.schema; const units = [...new Set(Object.values(schema?.properties || {}).map((field) => field['x-unit']).filter(Boolean))]; return <li key={port.name}><code>{port.name}</code> · {port.data_type}{units.length ? ` · ${units.join(', ')}` : ''}{port.description ? ` · ${port.description}` : ''}{port.transport ? ` · ${port.transport}` : ''}{port.delivery_mode ? ` · ${port.delivery_mode}` : ''}{port.contract ? ` · ${port.contract.owner}/${port.contract.id}@${port.contract.version}` : ' · static declaration'}{payload ? ` · ${payload.codec} · ${payload.media_type} · ${payload.max_payload_bytes} bytes · fields: ${Object.keys(schema?.properties || {}).join(', ') || '—'}` : ''}</li>; })}</ul></section>)}</aside>, document.body)}
    {configureOpen && createPortal(<aside className="node-graph-config-card nodrag nowheel" role="dialog" aria-label={`${data.lang === 'cn' ? '配置' : 'Configure'}: ${entry.node.label || entry.node.id}`} style={location} onPointerDown={(event) => event.stopPropagation()} onKeyDown={(event) => { if (event.key === 'Escape') { closeConfigure(); event.stopPropagation(); } }}><button ref={configCloseRef} type="button" className="node-graph-config-card__close" onClick={closeConfigure} aria-label={labels.close}>×</button><h3>{entry.node.label || entry.node.id}</h3><QuickNodeSettings data={data} /></aside>, document.body)}
  </div>;
}
const nodeTypes = { profile: FlowNode };
export function EdgeIdlPanel({ edge, entries, dataTypes = [], diagnostics, validated, processStatus, lang }: { edge: Edge; entries: GraphNode[]; dataTypes?: DataTypeDefinition[]; diagnostics: NodeGraphDiagnostic[]; validated: boolean; processStatus: ProcessStatus | null; lang: Language }): React.ReactElement {
  const { sourceType, targetType, sourcePort, targetPort, sourceIdl } = edgeContracts(entries, edge, dataTypes);
  const declaration = sourcePort?.delivery_mode === 'declaration' && targetPort?.delivery_mode === 'declaration';
  const protocol = sourcePort?.contract;
  const units = [...new Set(Object.values(sourceIdl?.schema?.properties || {}).map((field) => field['x-unit']).filter(Boolean))];
  const issues = validated ? edgeInputDiagnostics({ ...edge, inputIndex: edge.data?.inputIndex, inputCount: edge.data?.inputCount }, diagnostics) : [];
  return <section className="node-graph-edge-idl"><h2>{declaration ? (lang === 'cn' ? '静态拓扑连接' : 'Static topology link') : (lang === 'cn' ? '实时数据连接' : 'Live data connection')}</h2><p><code>{edge.source}.{sourcePort?.name || '—'}</code> → <code>{edge.target}.{targetPort?.name || '—'}</code></p>{declaration ? <p>{lang === 'cn' ? '此连接只声明节点顺序，不传输数据。' : 'This link declares node order and carries no data.'}</p> : <><p>{lang === 'cn' ? '通道' : 'Channel'}: {sourcePort?.transport || '—'} · {lang === 'cn' ? '传送方式' : 'Delivery'}: {sourcePort?.delivery_mode || '—'}</p><p>{lang === 'cn' ? '协议' : 'Protocol'}: {protocol ? `${protocol.owner}/${protocol.id}@${protocol.version}` : '—'}</p><p>{lang === 'cn' ? '数据类型' : 'Payload type'}: {sourceType || '—'}{units.length ? ` · ${units.join(', ')}` : ''}</p>{sourceIdl && <p>{sourceIdl.codec} · {sourceIdl.media_type} · {sourceIdl.max_payload_bytes} bytes · {lang === 'cn' ? '字段' : 'Fields'}: {Object.keys(sourceIdl.schema?.properties || {}).join(', ') || '—'}</p>}</>}{sourceType !== targetType && <p role="alert">{lang === 'cn' ? '端口类型不匹配。' : 'Port types do not match.'}</p>}{issues.map((issue, index) => <p role="alert" key={index}>{issue.message}</p>)}<p>{lang === 'cn' ? '部署状态' : 'Deployment state'}: {processStatus?.state || '—'}</p></section>;
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

function QuickNodeSettings({ data }: { data: FlowData }): React.ReactElement {
  const { entry, lang } = data;
  const fields = entry.definition?.settings || [];
  const basic = fields.filter((field) => field.required && field.default === undefined);
  const advanced = fields.filter((field) => !basic.includes(field));
  const change = (name: string, value: unknown): void => {
    const settings = { ...entry.node.settings };
    const field = fields.find((candidate) => candidate.name === name);
    if (value === undefined || value === '' && !field?.required || field?.default !== undefined && JSON.stringify(value) === JSON.stringify(field.default)) delete settings[name]; else settings[name] = value;
    data.onSettingsChange?.(entry.path, settings);
  };
  const renderField = (field: NodeTypeDefinition['settings'][number]): React.ReactElement => {
    const group = settingOptionGroup(data.options || null, entry.path, field.name);
    const effective = entry.node.settings?.[field.name] ?? field.default;
    const overridden = Object.prototype.hasOwnProperty.call(entry.node.settings || {}, field.name);
    return <div key={field.name} className="node-graph-config-card__field">
      {group ? <CatalogSelector group={group} label={field.name} value={String(effective ?? '')} lang={lang} pin={entry.node.package} help={field.description} onChange={(value) => change(field.name, value)} /> : field.selector ? <p role="status">{lang === 'cn' ? '选项尚未就绪；请刷新拥有方选项。' : 'Options are unavailable; refresh owner options.'}</p> : <SettingEditor name={field.name} dataType={field.data_type} required={field.required} description={field.description} unit={field.unit} min={field.min} max={field.max} choices={field.choices} value={effective} lang={lang} onChange={(value) => change(field.name, value)} />}
      {advanced.includes(field) && <div className="node-graph-config-card__default"><small>{field.default === undefined ? (overridden ? (lang === 'cn' ? '已设置可选值' : 'Optional override') : (lang === 'cn' ? '未设置可选值' : 'Optional, unset')) : `${overridden ? (lang === 'cn' ? '已覆盖默认值' : 'Override') : (lang === 'cn' ? '使用默认值' : 'Default')}: ${settingValue(field.default, field.data_type)}`}</small>{overridden && <button type="button" onClick={() => change(field.name, undefined)}>{lang === 'cn' ? '恢复默认' : 'Reset to default'}</button>}</div>}
    </div>;
  };
  return <fieldset disabled={data.readOnly} className="node-graph-readonly-fieldset"><h4>{lang === 'cn' ? '基本设置' : 'Basic'}</h4>{basic.map(renderField)}{entry.definition?.lifecycle === 'adapter' && <HardwareFields mode="basic" node={entry.node} path={entry.path} definition={entry.definition} options={data.options || null} lang={lang} onChange={(change) => data.onNodeChange?.(entry.path, change)} />}{!basic.length && entry.definition?.lifecycle !== 'adapter' && <p>{lang === 'cn' ? '无需填写；节点已就绪。' : 'Nothing required; this node is ready.'}</p>}<details><summary>{lang === 'cn' ? '高级设置' : 'Advanced'}</summary>{advanced.length ? advanced.map(renderField) : <p>{lang === 'cn' ? '没有其他目录设置。' : 'No additional catalog settings.'}</p>}{entry.definition?.lifecycle === 'adapter' && <HardwareFields mode="advanced" node={entry.node} path={entry.path} definition={entry.definition} options={data.options || null} lang={lang} onChange={(change) => data.onNodeChange?.(entry.path, change)} />}</details></fieldset>;
}

function HardwareFields({ node, path, definition, options, lang, onChange, mode = 'all' }: { node: NodeInstance; path: string; definition: NodeTypeDefinition; options: NodeOptions | null; lang: Language; onChange: (change: Partial<NodeInstance>) => void; mode?: 'all' | 'basic' | 'advanced' }): React.ReactElement {
  const adapter = node.adapter || { type: '', settings: {} };
  const adapterSettings = adapter.settings || {};
  const setAdapter = (key: string, value: unknown): void => onChange({ adapter: { ...adapter, settings: { ...adapterSettings, [key]: value } } });
  const adapterTypeGroup = settingOptionGroup(options, path, 'adapter.type');
  const deviceGroup = settingOptionGroup(options, path, 'adapter.settings.device_id');
  const simulatorGroup = settingOptionGroup(options, path, 'adapter.settings.simulator_node');
  const ownerDeviceOptions = adapter.type === 'mujoco-v2' || !!deviceGroup;
  const ownerSimulatorOptions = adapter.type === 'mujoco-v2' || !!simulatorGroup;
  const timing = node.health_timing;
  const observation = node.observation;
  const movement = node.movement;
  const limits = definition.kind === 'base' ? ['max_linear_m_s', 'max_angular_rad_s', 'max_velocity_lease_ms'] : definition.kind === 'manipulator' ? ['max_joint_position_rad', 'max_joint_speed_rad_s'] : [];
  return <div className="node-graph-hardware-fields">
    {mode !== 'advanced' && <>
    <h3>Adapter</h3>
    {adapterTypeGroup ? <CatalogSelector group={adapterTypeGroup} label={lang === 'cn' ? '适配器族' : 'Adapter family'} value={adapter.type} lang={lang} onChange={(value) => onChange({ adapter: { ...adapter, type: value } })} /> : <p role="status" className="node-graph-error">{lang === 'cn' ? '适配器选项尚未就绪。请检查 EOS 服务。' : 'Adapter options are unavailable. Check the EOS service.'}</p>}
    {(['backend', 'socket_path'] as const).map((key) => <SettingEditor key={`${node.id}:${key}`} name={key} dataType="string" required value={adapterSettings[key]} onChange={(value) => setAdapter(key, value)} />)}
    {deviceGroup ? <CatalogSelector group={deviceGroup} pin={node.adapter?.package} label="device_id" value={String(adapterSettings.device_id || '')} lang={lang} onChange={(value) => setAdapter('device_id', value)} /> : ownerDeviceOptions ? <p role="status" className="node-graph-error">{lang === 'cn' ? '设备选项尚未就绪。请检查 EOS 服务。' : 'Device options are unavailable. Check the EOS service.'}</p> : <SettingEditor name="device_id" dataType="string" required value={adapterSettings.device_id} onChange={(value) => setAdapter('device_id', value)} />}
    {simulatorGroup ? <CatalogSelector group={simulatorGroup} pin={node.adapter?.package} label="simulator_node" value={String(adapterSettings.simulator_node || '')} lang={lang} onChange={(value) => setAdapter('simulator_node', value)} /> : ownerSimulatorOptions ? <p role="status" className="node-graph-error">{lang === 'cn' ? '模拟器节点选项尚未就绪。请检查 EOS 服务。' : 'Simulator node options are unavailable. Check the EOS service.'}</p> : adapterSettings.simulator_node !== undefined ? <SettingEditor name="simulator_node" dataType="string" required value={adapterSettings.simulator_node} onChange={(value) => setAdapter('simulator_node', value)} /> : null}
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

function NodeInspector({ entry, entries, lang, options, runId, activeRevision, currentRun, onChange, onBind, onRename, onRemove }: { entry: GraphNode; entries: GraphNode[]; lang: Language; options: NodeOptions | null; runId: string | null; activeRevision: string | null; currentRun: boolean; onChange: (change: Partial<NodeInstance>) => void; onBind: (target: string) => void; onRename: (id: string) => void; onRemove: () => void }): React.ReactElement {
  const copy = COPY[lang];
  const [id, setId] = useState(entry.node.id);
  const [raw, setRaw] = useState('');
  const [error, setError] = useState('');
  const bindingTarget = entries.find((target) => { const value = target.node.inputs?.command; return value && inputSources(value).some((source) => sameNodePath(source.node, entry.path)); })?.path || '';
  useEffect(() => { setId(entry.node.id); setRaw(JSON.stringify(Object.fromEntries(Object.entries(entry.node).filter(([key]) => (DETAIL_FIELDS as readonly string[]).includes(key))), null, 2)); setError(''); }, [entry.path, entry.node]);
  const commitDetails = (): void => { try { const parsed: unknown = JSON.parse(raw); if (!parsed || Array.isArray(parsed) || typeof parsed !== 'object' || Object.keys(parsed).some((key) => !(DETAIL_FIELDS as readonly string[]).includes(key))) throw new Error(copy.invalid); const details = parsed as Partial<NodeInstance>; for (const field of DETAIL_FIELDS) if (!(field in details) && field in entry.node) Object.assign(details, { [field]: undefined }); onChange(details); setError(''); } catch { setError(copy.invalid); } };
  return <section className="node-graph-inspector">
    <h2>{copy.inspector}</h2><p><strong>{lang === 'cn' ? '类型描述' : 'Type description'}:</strong> {entry.definition?.description || (lang === 'cn' ? '当前目录没有类型描述。' : 'No type description in this catalog.')}</p><p><strong>{copy.type}:</strong> {entry.node.node_type}</p><p><strong>{copy.lifecycle}:</strong> {entry.definition?.lifecycle || '—'}</p>
    <label>{copy.id}<input value={id} onChange={(event) => setId(event.target.value.toLowerCase())} onBlur={() => { if (id !== entry.node.id) onRename(id.trim()); }} onKeyDown={(event) => { if (event.key === 'Enter') event.currentTarget.blur(); }} /></label>
    <label>{copy.label}<input value={entry.node.label} onChange={(event) => onChange({ label: event.target.value })} /></label>
    <label>{lang === 'cn' ? '实例描述' : 'Instance description'}<textarea rows={2} value={entry.node.description} onChange={(event) => onChange({ description: event.target.value })} /></label>
    <h3>{copy.capabilities}</h3><div className="node-graph-capabilities">{entry.definition?.capabilities.map((capability) => <label key={capability}><input type="checkbox" checked={entry.node.capabilities?.includes(capability) || false} onChange={(event) => onChange({ capabilities: event.target.checked ? [...(entry.node.capabilities || []), capability] : (entry.node.capabilities || []).filter((item) => item !== capability) })} />{capability}</label>)}</div>
    {(/^(?:eos\.dispatcher|isos\.eos_dispatcher)\.primitive_/.test(entry.node.node_type)) && <section className="node-graph-binding"><h3>{lang === 'cn' ? '原语绑定' : 'Primitive binding'}</h3><p>{options?.primitive_bindings.find((option) => option.node_type === entry.node.node_type)?.reason || ''}</p><label>{lang === 'cn' ? '目标硬件' : 'Target hardware'}<select value={bindingTarget} onChange={(event) => onBind(event.target.value)}><option value="">—</option>{bindingTarget && !options?.targets.some((target) => sameNodePath(target.path, bindingTarget)) && <option value={bindingTarget}>{bindingTarget}</option>}{options?.targets.filter((target) => target.device_kind === (entry.node.node_type.includes('_base') ? 'base' : 'manipulator')).map((target) => <option key={target.path} value={target.path} disabled={!target.available}>{target.path}{target.available ? '' : ` — ${target.reason || 'unavailable'}`}</option>)}</select></label>{!options && <p role="status">{lang === 'cn' ? '节点选项服务不可用' : 'Node options service unavailable'}</p>}</section>}

    {!!entry.definition?.settings.length && <><h3>{copy.settings}</h3>{entry.definition.settings.map((field) => {
      const group = settingOptionGroup(options, entry.path, field.name);
      const update = (value: unknown): void => {
        const settings = { ...entry.node.settings };
        if (value === undefined) delete settings[field.name]; else settings[field.name] = value;
        onChange({ settings });
      };
      if (field.selector?.provider === 'ui') return <p key={`${entry.path}:${field.name}`} role="alert" className="node-graph-error">{lang === 'cn' ? '当前图不支持浏览器输入选择器。' : 'Browser input selectors are unsupported by the current graph.'}</p>;
      if (group) return <CatalogSelector key={`${entry.path}:${field.name}`} group={group} pin={entry.node.package} help={field.description} label={field.name} value={String(entry.node.settings?.[field.name] ?? field.default ?? '')} lang={lang} onChange={update} />;
      if (field.selector) return <div key={`${entry.path}:${field.name}`} className="node-graph-catalog-selector"><strong>{field.name}</strong>{field.description && <small>{field.description}</small>}<p role="status" className="node-graph-error">{lang === 'cn' ? '此节点的可用选项尚未就绪。请检查拥有方服务，然后刷新选项。' : 'Options for this node are unavailable. Check the owner service, then refresh options.'}</p></div>;
      return <SettingEditor key={`${entry.path}:${field.name}`} lang={lang} name={field.name} dataType={field.data_type} required={field.required} description={field.description} unit={field.unit} min={field.min} max={field.max} choices={field.choices} value={entry.node.settings?.[field.name] ?? field.default} onChange={update} />;
    })}</>}
    {entry.definition?.lifecycle === 'adapter' && <HardwareFields node={entry.node} path={entry.path} definition={entry.definition} options={options} lang={lang} onChange={onChange} />}
    {entry.definition?.lifecycle === 'adapter' && <><h3>{copy.details}</h3><textarea rows={12} value={raw} onChange={(event) => setRaw(event.target.value)} /><button type="button" onClick={commitDetails}>{copy.applyDetails}</button></>}
    {error && <p role="alert" className="node-graph-error">{error}</p>}<button type="button" className="node-graph-remove" onClick={onRemove}>{copy.remove}</button>
  </section>;
}

export function NodeGraphManager({ lang, visible = true, onDirtyChange, onOpenPanel, onStopIntent, onStopOutcome }: Props): React.ReactElement {
  const copy = COPY[lang];
  const [graph, setGraph] = useState<NodeGraph | null>(null);
  const [rawGraph, setRawGraph] = useState<unknown>(null);
  const [saved, setSaved] = useState('');
  const [revision, setRevision] = useState('');
  const [readOnlyReason, setReadOnlyReason] = useState<string | null>(null);
  const [types, setTypes] = useState<NodeTypeDefinition[]>([]);
  const [dataTypes, setDataTypes] = useState<DataTypeDefinition[]>([]);
  const [nodeSets, setNodeSets] = useState<NodeSetDefinition[]>([]);
  const [setKey, setSetKey] = useState('');
  const [setValues, setSetValues] = useState<Record<string, string>>({});
  const [setPreview, setSetPreview] = useState<{ result: NodeSetPreview; source: string } | null>(null);
  const [processStatus, setProcessStatus] = useState<ProcessStatus | null>(null);
  const [selected, setSelected] = useState<string[]>([]);
  const [typeId, setTypeId] = useState('');
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
  const lastCanvasSize = useRef('');
  const canvasRef = useRef<HTMLDivElement>(null);
  const quickSearchRef = useRef<HTMLInputElement>(null);
  const overviewTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [undoStack, setUndoStack] = useState<NodeGraph[]>([]);
  const [redoStack, setRedoStack] = useState<NodeGraph[]>([]);
  const [diagnostics, setDiagnostics] = useState<NodeGraphDiagnostic[]>([]);
  const [validationStamp, setValidationStamp] = useState<DraftValidationStamp | null>(null);
  const [validatedEdges, setValidatedEdges] = useState<ValidatedEdge[]>([]);
  const [edgeInputInvalid, setEdgeInputInvalid] = useState(false);
  const [diagnosticsOpen, setDiagnosticsOpen] = useState(false);
  const [frontNode, setFrontNode] = useState<string | null>(null);
  const setMessage = (text: string, error = false): void => { publishUiMessage(text, error ? 'error' : 'info'); };
  const [optionSnapshot, setOptionSnapshot] = useState<OptionSnapshot | null>(null);
  const optionRequest = useRef(0);
  const catalogReloadRequest = useRef(0);
  const statusRequest = useRef(0);
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
  const [mirrors, setMirrors] = useState<NodeMirrors>({});
  const [viewOpen, setViewOpen] = useState(false);
  const [coreWindows, setCoreWindows] = useState<Record<'ei', CoreWindowState>>(() => ({ ei: readCoreWindow('isosui-graph-ei-window', 190) }));
  const dirty = Boolean(graph && JSON.stringify(graph) !== saved);
  const readOnly = readOnlyReason !== null;
  const draftKey = graph ? JSON.stringify(graph) : '';
  const currentDraft = useRef(draftKey);
  currentDraft.current = draftKey;
  const nodeOptions = optionSnapshot?.draft === draftKey ? optionSnapshot.options : null;
  useEffect(() => {
    const requestId = ++optionRequest.current;
    if (!graph || readOnly) return;
    const draft = JSON.stringify(graph);
    const timer = window.setTimeout(() => { void getNodeOptions(graph).then((options) => {
      if (optionResponseIsCurrent(requestId, optionRequest.current, draft, currentDraft.current)) setOptionSnapshot({ draft, options });
    }).catch(() => { if (requestId === optionRequest.current) setOptionSnapshot(null); }); }, 250);
    return () => { window.clearTimeout(timer); optionRequest.current += 1; };
  }, [graph, readOnly, packageGeneration]);
  useEffect(() => { onDirtyChange?.(dirty); }, [dirty, onDirtyChange]);
  const entries = useMemo(() => graph ? graphNodes(graph, types) : [], [graph, types]);
  const monitorRateHz = useMemo(() => { try { return monitorRefreshHz(saved ? JSON.parse(saved) as NodeGraph : null); } catch { return DEFAULT_MONITOR_REFRESH_HZ; } }, [saved]);
  const monitorReading = useArmMonitor(visible && entries.some((entry) => entry.node.node_type === 'isos.ui.monitor.arm@v0.2'), monitorRateHz);
  const selectorBlocked = catalogSelectionBlocks(graph?.version, entries, nodeOptions);
  useEffect(() => { if (graph) browserLog('debug', `Graph revision ${revision.slice(0, 8) || 'unsaved'}: ${entries.length} nodes; visible=${visible}`); }, [graph, revision, visible]);
  useEffect(() => {
    if (!visible || !flowInstance || !entries.length) return;
    const canvas = canvasRef.current;
    if (!canvas) return;
    const observer = new ResizeObserver(() => {
      const size = `${canvas.clientWidth}x${canvas.clientHeight}`;
      if (size === lastCanvasSize.current) return;
      lastCanvasSize.current = size;
      browserLog(canvas.clientWidth && canvas.clientHeight ? 'debug' : 'warn', `Graph canvas ${size}; viewport ${JSON.stringify(flowInstance.getViewport())}`);
    });
    observer.observe(canvas);
    return () => observer.disconnect();
  }, [visible, flowInstance, entries.length]);
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
  const quickTypes = useMemo(() => availableTypes.filter((type) => `${type.id} ${type.core} ${type.layer} ${type.group} ${type.kind}`.toLowerCase().includes(quickSearch.toLowerCase())), [availableTypes, quickSearch]);
  const selectedSet = nodeSets.find((set) => `${set.package.id}@${set.package.version}/${set.id}` === setKey);
  useEffect(() => { if (quickAdd) quickSearchRef.current?.focus(); }, [quickAdd]);
  useEffect(() => { if (diagnostics.length) setDiagnosticsOpen(true); }, [diagnostics]);
  useEffect(() => () => { if (overviewTimer.current) clearTimeout(overviewTimer.current); }, []);
  const revealOverview = (): void => {
    setShowOverview(true);
    if (overviewTimer.current) clearTimeout(overviewTimer.current);
    overviewTimer.current = setTimeout(() => setShowOverview(false), 1800);
  };
  const chosen = selected.length === 1 ? entries.find((entry) => entry.path === selected[0]) : undefined;
  const refreshPool = async (): Promise<void> => {
    try { setPoolEntries((await getGraphPool()).graphs); }
    catch (error) { setMessage(error instanceof Error ? error.message : String(error), true); }
  };
  const loadPoolGraph = async (name = poolSelected): Promise<void> => {
    if (!name) return;
    if (dirty && !window.confirm(copy.discard)) return;
    setBusy(true);
    try {
      const [entry, snapshot] = await Promise.all([getGraphPoolEntry(name), getGraphCatalogSnapshot()]);
      if (!nodeGraphHasEditableShape(entry.graph) || entry.graph.version !== '0.2.0') throw new Error('Selected pool graph is not an editable 0.2.0 graph.');
      const catalog = combineNodeTypeCatalogs(snapshot.selected, snapshot.global);
      const sameAsSaved = JSON.stringify(entry.graph) === JSON.stringify(snapshot.graph.graph);
      const poolReadOnlyReason = sameAsSaved
        ? graphCatalogReadOnlyReason(snapshot)
        : snapshot.reason || nodeGraphReadOnlyReason({ ...snapshot.graph, graph: entry.graph, read_only: false, diagnostics: [] }, catalog.types);
      editGeneration.current += 1;
      catalogReloadRequest.current += 1;
      setGraph(entry.graph); setRawGraph(null); setRevision(entry.revision); setSaved(JSON.stringify(entry.graph));
      setNewEmptyDraft(false); setReadOnlyReason(poolReadOnlyReason); setGraphLoadError('');
      setTypes(catalog.types); setDataTypes(catalog.data_types || []);
      setDiagnostics([]); setValidatedEdges([]); setValidationStamp(null);
      setSelected([]); setUndoStack([]); setRedoStack([]);
      setPositions(parseNodePositions(localStorage.getItem(graphLayoutKey(entry.graph))));
      setMirrors(parseNodeMirrors(localStorage.getItem(graphMirrorKey(entry.graph))));
      layoutKeyRef.current = graphLayoutKey(entry.graph);
      setOptionSnapshot(null);
      setPoolRevision(entry.revision);
      setPoolName(entry.name);
      setPoolSelected(entry.name);
      setFallbackNotice('');
      localStorage.setItem('isosui-graph-pool-last-name', entry.name);
      setMessage(lang === 'cn' ? `已打开图库文件“${entry.name}”。` : `Opened graph file “${entry.name}”.`);
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
  const newDemoGraph = async (): Promise<void> => {
    if (busy) return;
    if (dirty && !window.confirm(copy.discard)) return;
    setBusy(true);
    try {
      const document = await getNewNodeGraph();
      if (!nodeGraphHasEditableShape(document.graph) || document.graph.version !== '0.2.0') throw new Error('IsOS did not provide an editable 0.2.0 graph.');
      const catalog = await getNodeTypes();
      editGeneration.current += 1;
      catalogReloadRequest.current += 1;
      setGraph(document.graph);
      setRawGraph(null);
      setRevision(document.revision);
      setSaved('');
      setPoolName(''); setPoolRevision('');
      setNewEmptyDraft(true);
      setTypes(catalog.types); setDataTypes(catalog.data_types || []);
      setReadOnlyReason(null); setGraphLoadError('');
      setDiagnostics([]); setValidatedEdges([]); setValidationStamp(null);
      setSelected([]); setUndoStack([]); setRedoStack([]);
      setPositions({}); setMirrors({}); setOptionSnapshot(null);
      setMessage(lang === 'cn' ? '已载入当前节点演示草稿。保存后才会替换已保存的节点图。' : 'Current node demo loaded as a draft. Save it to replace the saved graph.');
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
      setDataTypes(catalog?.data_types || []);
      setNodeSets([]);
      setSetKey(''); setSetValues({}); setSetPreview(null);
      void getNodeSets().then((sets) => { if (catalogRequestId === catalogReloadRequest.current && generation === editGeneration.current) setNodeSets(sets.sets); }).catch(() => undefined);
      void getProcessStatus().then((status) => { if (catalogRequestId === catalogReloadRequest.current && statusRequestId === statusRequest.current) setProcessStatus(status); }).catch(() => undefined);
      void getCoreReadiness().then((cores) => { if (catalogRequestId === catalogReloadRequest.current && readinessRequestId === readinessRequest.current) setReadiness(cores); }).catch(() => undefined);
      setTypeId('');
      setSelected((current) => shaped ? current.filter((path) => graphNodes(document.graph, []).some((entry) => entry.path === path)) : []);
      setDiagnostics(document.diagnostics || []);
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
    const statusTimer = window.setInterval(() => {
      if (document.visibilityState !== 'visible' || statusPending) return;
      statusPending = true;
      const requestId = ++statusRequest.current;
      void getProcessStatus().then((status) => { if (active && requestId === statusRequest.current) setProcessStatus(status); })
        .catch(() => undefined).finally(() => { statusPending = false; });
    }, 3000);
    const readinessTimer = window.setInterval(() => {
      if (document.visibilityState !== 'visible' || readinessPending) return;
      readinessPending = true;
      const requestId = ++readinessRequest.current;
      void getCoreReadiness().then((readiness) => { if (active && requestId === readinessRequest.current) setReadiness(readiness); })
        .catch(() => undefined).finally(() => { readinessPending = false; });
    }, 30000);
    return () => { active = false; ++statusRequest.current; ++readinessRequest.current; window.clearInterval(statusTimer); window.clearInterval(readinessTimer); };
  }, [visible]);
  useEffect(() => {
    const reloadCatalog = (): void => { setValidatedEdges([]); const requestId = ++catalogReloadRequest.current; optionRequest.current += 1; setOptionSnapshot(null); setPackageGeneration((value) => value + 1); void Promise.all([getGraphCatalogSnapshot(), getNodeSets()]).then(([snapshot, sets]) => {
      if (!catalogReloadIsCurrent(requestId, catalogReloadRequest.current)) return;
      const { graph: document } = snapshot;
      const catalog = combineNodeTypeCatalogs(snapshot.selected, snapshot.global);
      setTypes(catalog.types); setDataTypes(catalog.data_types || []); setNodeSets(sets.sets); setSetKey(''); setSetValues({}); setSetPreview(null);
      const currentGraph = catalogReloadGraph(document.revision, revisionForCatalog.current, graphForCatalog.current, document.graph);
      const savedReason = graphCatalogReadOnlyReason(snapshot);
      setReadOnlyReason(savedReason || (currentGraph
        ? nodeGraphReadOnlyReason({ ...document, graph: currentGraph }, catalog.types)
        : 'Saved graph revision changed; refresh before editing.'));
    }).catch((error: unknown) => { if (catalogReloadIsCurrent(requestId, catalogReloadRequest.current)) { setReadOnlyReason('Graph catalog could not be checked.'); setMessage(error instanceof Error ? error.message : String(error)); } }); };
    window.addEventListener('isosui:packages-changed', reloadCatalog);
    return () => { window.removeEventListener('isosui:packages-changed', reloadCatalog); catalogReloadRequest.current += 1; };
  }, []);
  useEffect(() => { if (!graph) return; const key = graphLayoutKey(graph); const timer = window.setTimeout(() => localStorage.setItem(key, JSON.stringify(positions)), 250); return () => window.clearTimeout(timer); }, [graph?.robot.id, positions]);
  useEffect(() => { if (graph) localStorage.setItem(graphMirrorKey(graph), JSON.stringify(mirrors)); }, [graph?.robot.id, mirrors]);
  useEffect(() => { for (const id of ['ei'] as const) localStorage.setItem(`isosui-graph-${id}-window`, JSON.stringify(coreWindows[id])); }, [coreWindows]);
  const updateCoreWindow = (id: 'ei', patch: Partial<CoreWindowState>): void => setCoreWindows((current) => ({ ...current, [id]: { ...current[id], ...patch } }));
  useEffect(() => { const beforeUnload = (event: BeforeUnloadEvent): void => { if (dirty) event.preventDefault(); }; window.addEventListener('beforeunload', beforeUnload); return () => window.removeEventListener('beforeunload', beforeUnload); }, [dirty]);
  const edit = (change: (current: NodeGraph) => NodeGraph): void => {
    if (!graph || readOnly) return;
    const next = change(graph);
    if (next === graph || JSON.stringify(next) === JSON.stringify(graph)) return;
    editGeneration.current += 1; setUndoStack((stack) => [...stack.slice(-49), graph]); setRedoStack([]); setGraph(next); setSetPreview(null); setValidatedEdges([]); setDiagnostics([]); setValidationStamp(null);
  };
  const undo = (): void => { if (!graph || readOnly || !undoStack.length) return; editGeneration.current += 1; setRedoStack((stack) => [...stack, graph]); setGraph(undoStack[undoStack.length - 1]); setSetPreview(null); setDiagnostics([]); setValidationStamp(null); setValidatedEdges([]); setUndoStack(undoStack.slice(0, -1)); };
  const redo = (): void => { if (!graph || readOnly || !redoStack.length) return; editGeneration.current += 1; setUndoStack((stack) => [...stack, graph]); setGraph(redoStack[redoStack.length - 1]); setSetPreview(null); setDiagnostics([]); setValidationStamp(null); setValidatedEdges([]); setRedoStack(redoStack.slice(0, -1)); };
  const previewPackageSet = async (): Promise<void> => {
    if (!graph || readOnly || !selectedSet) return;
    setBusy(true);
    try {
      const source = JSON.stringify(graph);
      const result = await previewNodeSet(selectedSet, graph, setValues);
      if (removedNodePaths(graph, result.graph).length) throw new Error(copy.previewRemoves);
      setSetPreview({ result, source });
      setMessage(`${result.added_paths.length} ${copy.previewCount}.`);
    }
    catch (error) { setSetPreview(null); setMessage(error instanceof Error ? error.message : String(error), true); }
    finally { setBusy(false); }
  };
  const addPreviewedSet = (): void => {
    if (!graph || readOnly || !setPreview || setPreview.source !== JSON.stringify(graph)) { setMessage(copy.previewStale, true); setSetPreview(null); return; }
    edit(() => setPreview.result.graph);
    setSelected(setPreview.result.added_paths.map(canonicalNodePath));
    setMessage(`${setPreview.result.added_paths.length} ${copy.previewCount}.`);
    setSetPreview(null);
  };
  const add = (automatic = false): void => { const definition = availableTypes.find((type) => catalogTypeKey(type) === typeId); if (!definition || !graph || readOnly) return; const id = automatic || !newId.trim() ? nextNodeId(graph, definition) : newId.trim(); try { edit((current) => addNode(current, definition, id)); setSelected([nodePath(definition, id)]); setNewId(''); setMessage(''); } catch (error) { setMessage(error instanceof Error ? error.message : copy.duplicate, true); } };
  const addAtCursor = (definition: NodeTypeDefinition): void => {
    if (!quickAdd || !graph || readOnly || definition.deprecated) return;
    const id = nextNodeId(graph, definition);
    try {
      edit((current) => addNode(current, definition, id));
      const path = nodePath(definition, id);
      setPositions((current) => ({ ...current, [path]: { x: quickAdd.x, y: quickAdd.y } }));
      setSelected([path]); setQuickAdd(null); setMessage('');
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
    else if ((event.key === 'Delete' || event.key === 'Backspace') && selected.length) { event.preventDefault(); edit((current) => removeSelectedNodes(current, selected)); setPositions((current) => Object.fromEntries(Object.entries(current).filter(([path]) => !selected.includes(path)))); setSelected([]); }
    else if (event.key.toLowerCase() === 'r' && !event.ctrlKey && !event.metaKey && !quickAdd && !edgeMenu && selected.length) { event.preventDefault(); setMirrors((current) => { const next = { ...current }; for (const path of selected) next[path] = !(next[path] ?? asyncDemoLayout(entries.map((entry) => entry.path))?.mirrors[path] ?? false); return next; }); }
  }; window.addEventListener('keydown', onKey); return () => window.removeEventListener('keydown', onKey); }, [graph, undoStack, redoStack, selected, quickAdd, edgeMenu, visible, readOnly]);
  const validate = async (): Promise<boolean> => { if (!graph || readOnly) return false; const generation = editGeneration.current; setBusy(true); try { const result = await validateNodeGraph(graph); if (!validationResponseIsCurrent(generation, editGeneration.current)) return false; setDiagnostics(result.diagnostics); setValidatedEdges(result.valid && Array.isArray(result.edges) ? result.edges : []); setValidationStamp(draftValidationStamp(graph, generation)); setMessage(result.valid ? copy.valid : `${result.diagnostics.length} ${copy.diagnostics.toLowerCase()}`, !result.valid); return result.valid; } catch (error) { if (validationResponseIsCurrent(generation, editGeneration.current)) { setValidationStamp(null); setMessage(error instanceof Error ? error.message : String(error), true); } return false; } finally { setBusy(false); } };
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
    setTypes(catalog.types); setDataTypes(catalog.data_types || []);
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
    const generation = editGeneration.current;
    const stopAtStart = deploymentStopEpoch();
    try {
      const nextRevision = await saveSnapshot(graph);
      if (!nextRevision) return;
      if (generation !== editGeneration.current || stopAtStart !== deploymentStopEpoch()) { setMessage(lang === 'cn' ? '运行前节点图或停止状态已更改，请再次检查。' : 'Graph or Stop state changed before run. Check again.', true); return; }
      const latestStatus = await getProcessStatus();
      if (generation !== editGeneration.current || stopAtStart !== deploymentStopEpoch()) { setMessage(lang === 'cn' ? '运行前节点图或停止状态已更改，请再次检查。' : 'Graph or Stop state changed before run. Check again.', true); return; }
      const operation = hasActiveDeployment(latestStatus) ? 'restart' : 'run';
      const status = await runDeployment(operation, nextRevision);
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
  const nodes = useMemo<Node<FlowData>[]>(() => entries.map((entry, index) => ({ id: entry.path, type: 'profile', position: positions[entry.path] || demoLayout?.positions[entry.path] || { x: 50 + (index % 3) * 330, y: 50 + Math.floor(index / 3) * 220 }, zIndex: frontNode === entry.path ? 20 : selected.includes(entry.path) ? 1 : 0, data: { entry, error: diagnostics.some((item) => item.path.includes(entry.path)), mirrored: mirrors[entry.path] ?? demoLayout?.mirrors[entry.path] ?? false, lane: demoLayout?.lanes[entry.path], lang, dataTypes, options: nodeOptions, readOnly, currentRun: !dirty && !readOnly && processStatus?.state === 'running' && processStatus.run_id_state === 'known' && processStatus.active_revision === revision && processStatus.saved_revision === revision, runId: !processStatus ? undefined : processStatus.state === 'running' || processStatus.state === 'degraded' ? processStatus.run_id || null : null, activeRevision: !processStatus ? undefined : processStatus.state === 'running' || processStatus.state === 'degraded' ? processStatus.active_revision || null : null, monitorReading, onSettingsChange: (path, settings) => edit((current) => updateNode(current, path, { settings })), onNodeChange: (path, change) => edit((current) => updateNode(current, path, change)) }, selected: selected.includes(entry.path) })), [entries, positions, selected, frontNode, diagnostics, mirrors, demoLayout, lang, dataTypes, dirty, readOnly, processStatus, revision, nodeOptions, monitorReading]);
  const edges = useMemo<Edge[]>(() => entries.flatMap((entry) => Object.entries(entry.node.inputs || {}).flatMap(([input, sources]) => {
    const sourceList = inputSources(sources);
    return sourceList.map((source, inputIndex) => {
    const port = entries.find((candidate) => sameNodePath(candidate.path, source.node))?.definition?.outputs.find((item) => item.name === source.output);
    const admitted = validationCurrent ? validatedEdges.find((candidate) => candidate.source === canonicalNodePath(source.node) && candidate.output === source.output && candidate.target === entry.path && candidate.input === input) : undefined;
    const visual = interfaceVisual(port, admitted ? admitted.frequency_hz : source.frequency_hz, admitted ? admitted.priority : source.priority ?? 0);
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
  const onNodesChange = (changes: NodeChange[]): void => {
    if (changes.some((change) => change.type === 'select')) setSelected((current) => applyNodeSelection(current, changes));
    setPositions((current) => {
    const moved = changes.filter((change): change is Extract<NodeChange, { type: 'position' }> => change.type === 'position' && Boolean(change.position));
    if (!moved.length) return current;
    const next = { ...current };
    let changed = false;
    for (const change of moved) if (change.position && (current[change.id]?.x !== change.position.x || current[change.id]?.y !== change.position.y)) { next[change.id] = change.position; changed = true; }
    return changed ? next : current;
    });
  };
  return <ReactFlowProvider><main className="node-graph-manager">
    <header><h1>{copy.title}</h1><div role="toolbar" aria-label={copy.title}>
      <button className="node-graph-icon-button" type="button" onClick={undo} disabled={readOnly || !undoStack.length || busy} title={`${copy.undo} (Ctrl+Z)`} aria-label={copy.undo}><span className="node-graph-history-arrow" aria-hidden="true" /></button>
      <button className="node-graph-icon-button" type="button" onClick={redo} disabled={readOnly || !redoStack.length || busy} title={`${copy.redo} (Ctrl+Shift+Z)`} aria-label={copy.redo}><span className="node-graph-history-arrow node-graph-history-arrow--redo" aria-hidden="true" /></button>
      <div className="node-graph-view"><button type="button" aria-expanded={viewOpen} onClick={() => setViewOpen((value) => !value)}>{lang === 'cn' ? '视图' : 'View'} ▾</button>{viewOpen && <div className="node-graph-view__menu" role="menu"><button type="button" role="menuitemcheckbox" aria-checked={!libraryCollapsed} onClick={() => setLibraryCollapsed((value) => !value)}>{copy.catalog}</button><button type="button" role="menuitemcheckbox" aria-checked={!sideCollapsed} onClick={() => setSideCollapsed((value) => !value)}>{copy.inspector}</button><button type="button" role="menuitemcheckbox" aria-checked={diagnosticsOpen && !sideCollapsed} onClick={() => { if (diagnosticsOpen && !sideCollapsed) setDiagnosticsOpen(false); else { setSideCollapsed(false); setDiagnosticsOpen(true); } }}>{copy.diagnostics}</button><button type="button" role="menuitem" onClick={() => onOpenPanel?.('logs')}>{lang === 'cn' ? '日志' : 'Logs'}</button><button type="button" role="menuitem" onClick={() => onOpenPanel?.('control')}>EOS</button><button type="button" role="menuitemcheckbox" aria-checked={coreWindows.ei.open} onClick={() => updateCoreWindow('ei', { open: !coreWindows.ei.open })}>EI</button></div>}</div>
      <button type="button" onClick={() => { browserLog('info', `Fit graph requested; ${entries.length} nodes; viewport ${JSON.stringify(flowInstance?.getViewport())}`); setViewLost(false); void flowInstance?.fitView({ padding: 0.15, duration: 200 }); }}>{lang === 'cn' ? '显示全图' : 'Fit graph'}</button>
      <button type="button" className="node-graph-primary-action" onClick={() => void runPrimary()} disabled={readOnly || busy || !graph || selectorBlocked || edgeInputInvalid} title={lang === 'cn' ? '保存更改（如有）并启动；运行中则重启' : 'Save changes if needed, then start or restart the deployment'}>{newEmptyDraft ? copy.apply : primaryLabel}</button>
      {(running || busy) && <button type="button" onClick={() => void stop()} disabled={stopping}>{lang === 'cn' ? '停止' : 'Stop'}</button>}
      <details className="node-graph-advanced" open={advancedOpen} onToggle={(event) => setAdvancedOpen(event.currentTarget.open)}><summary>{lang === 'cn' ? '高级操作' : 'Advanced'} ▾</summary><div>
        <button type="button" onClick={() => void validate()} disabled={readOnly || busy || !graph}>{copy.validate}</button><small>{lang === 'cn' ? '检查草稿，不保存' : 'Check draft without saving'}</small>
        <button type="button" onClick={() => void apply()} disabled={readOnly || busy || !graph || selectorBlocked || edgeInputInvalid}>{copy.apply}</button><small>{lang === 'cn' ? '检查并应用图库文件，同时生成核心配置' : 'Check and Apply the graph file; generate core configuration'}</small>
        <button type="button" onClick={() => void (poolName ? loadPoolGraph(poolName) : refresh(true))} disabled={busy}>{lang === 'cn' ? '重新载入当前文件' : 'Reload current file'}</button><small>{lang === 'cn' ? '丢弃修改并重新读取当前图库文件' : 'Discard edits and reread the current graph file'}</small>
        <button type="button" onClick={() => void newDemoGraph()} disabled={busy}>{lang === 'cn' ? '载入当前节点演示' : 'Load current node demo'}</button><small>{lang === 'cn' ? '载入当前演示草稿；保存后才会替换当前节点图' : 'Load the current demo as a draft; the saved graph changes only when you save it'}</small>
        <label>{lang === 'cn' ? '图库文件' : 'Graph file'}<select value={poolSelected} onChange={(event) => { setPoolSelected(event.target.value); }}><option value="">—</option>{poolEntries.map((entry) => <option key={entry.name} value={entry.name}>{entry.name}</option>)}</select></label>
        <small>{poolName ? (lang === 'cn' ? `当前编辑：${poolName}${poolFileExists ? '。保存将替换同名文件。' : '。原文件不存在；保存时需要新文件名。'}` : `Editing: ${poolName}${poolFileExists ? '. Save will replace the file with the same name.' : '. The file is missing; Save will ask for a new name.'}`) : (lang === 'cn' ? '尚未绑定图库文件；保存时需要文件名。' : 'No graph file is bound; Save will ask for a name.')}</small>
        <button type="button" onClick={() => void refreshPool()} disabled={busy}>{lang === 'cn' ? '刷新图库' : 'Refresh pool'}</button>
        <button type="button" onClick={() => void loadPoolGraph()} disabled={busy || !poolSelected}>{lang === 'cn' ? '打开文件' : 'Open file'}</button>
        <button type="button" onClick={() => void savePoolGraph()} disabled={busy || !graph}>{lang === 'cn' ? '保存文件' : 'Save file'}</button><small>{lang === 'cn' ? '保存文件后，应用时才更新部署配置' : 'Save the file, then Apply to update deployment configuration'}</small>
      </div></details>
      <span role="status">{dirty ? copy.dirty : copy.clean} · {copy.savedRevision}: {revision.slice(0, 8) || '—'} · {copy.activeRevision}: {processStatus?.active_revision?.slice(0, 8) || '—'}</span>
    </div></header>
    {selectorBlocked && <p className="node-graph-error" role="status">{lang === 'cn' ? '当前草稿的目录选项尚未就绪，或已保存选项不可用。检查图仍可使用；修复后才能保存或运行。' : 'Catalog options for this draft are pending or a saved choice is unavailable. Check graph remains available; repair the choice before saving or running.'}</p>}
    {fallbackNotice && <p className="node-graph-note" role="status">{fallbackNotice}</p>}
    {readOnlyReason && <p className="node-graph-error" role="alert">{lang === 'cn' ? '只读节点图：' : 'Read-only graph: '}{readOnlyReason}</p>}
    {graphLoadError && <p className="node-graph-error" role="alert">{lang === 'cn' ? '无法打开已保存的节点图。' : 'The saved graph cannot be opened.'} <button type="button" onClick={() => void newDemoGraph()} disabled={busy}>{lang === 'cn' ? '载入当前节点演示' : 'Load current node demo'}</button></p>}
    {readiness?.cores.map((core) => {
      const disabled = core.enabled === false;
      const alert = !disabled && !core.ready;
      return <p key={core.core} className={alert ? 'node-graph-error' : 'node-graph-note'} role={alert ? 'alert' : 'status'}>
        <strong>{core.core.toUpperCase()}</strong>: {core.configured ? `${core.version || '—'} · ${core.path || '—'}` : (lang === 'cn' ? '未配置程序路径' : 'Binary path not configured')}
        {' · '}{disabled ? (lang === 'cn' ? '保存的节点图中未启用' : 'Not enabled in the saved graph') : (core.running ? (lang === 'cn' ? '运行中' : 'Running') : (lang === 'cn' ? '已停止' : 'Stopped'))}
        {!disabled && core.reason ? ` · ${core.reason === 'pending_apply' ? (lang === 'cn' ? '图库文件已保存，等待应用以更新部署配置' : 'Graph file saved; Apply to update deployment configuration') : core.reason}` : ''}{!disabled && core.action && core.reason !== 'pending_apply' ? ` · ${core.action}` : ''}
      </p>;
    })}
    {viewLost && <p className="node-graph-note" role="status">{lang === 'cn' ? '节点可能在视野之外。使用“显示全图”恢复。' : 'Nodes may be outside the view. Use Fit graph to recover.'}</p>}{processStatus?.last_error && <p className="node-graph-error" role="alert">{processStatus.last_error}</p>}
    <div className={`node-graph-body${libraryCollapsed ? ' node-graph-body--library-collapsed' : ''}${sideCollapsed ? ' node-graph-body--side-collapsed' : ''}`}><aside className="node-graph-library" hidden={libraryCollapsed}><h2>{copy.catalog}</h2><CatalogBrowser types={availableTypes} value={typeId} lang={lang} onChange={setTypeId} /><label>{copy.id}<input value={newId} placeholder={graph && availableTypes.find((type) => catalogTypeKey(type) === typeId) ? nextNodeId(graph, availableTypes.find((type) => catalogTypeKey(type) === typeId)!) : ''} onChange={(event) => setNewId(event.target.value.toLowerCase())} /></label><button type="button" onClick={() => add()} disabled={readOnly || !graph || !availableTypes.some((type) => catalogTypeKey(type) === typeId)}>{copy.add}</button><p>{copy.port}</p><p>{copy.hint}</p><div className="node-graph-legend" aria-label={lang === 'cn' ? '端口颜色' : 'Port colors'}><h3>{lang === 'cn' ? '端口颜色：准确数据类型' : 'Port color: exact data type'}</h3>{(dataTypes.length ? dataTypes.slice(0, 5).map((item) => item.id) : ['camera_frame', 'vision_frame', 'base_observation']).map((id) => <span key={id} style={{ '--port': dataTypeColor(id) } as React.CSSProperties}><i aria-hidden="true" />{id}</span>)}</div><div className="node-graph-interface-legend" aria-label={lang === 'cn' ? '接口视觉规则' : 'Interface visual rules'}><strong>{lang === 'cn' ? '接口视觉规则' : 'Interface visual rules'}</strong><span>{lang === 'cn' ? '端口形状表示通道：进程内圆形 / Unix 方形 / HTTP 盾形 / WebSocket 菱形 / ROS 2 六边形' : 'Port shape shows channel: in-process circle / Unix square / HTTP shield / WebSocket diamond / ROS 2 hexagon'}</span><span>{lang === 'cn' ? '连线图案表示连线上限：<1 Hz 稀疏点；1–<10 Hz 点线；10–<30 Hz 长划线；≥30 Hz 实线；无限制为长短交替线' : 'Line pattern shows edge cap: <1 Hz sparse dots; 1–<10 Hz dotted; 10–<30 Hz long dashes; ≥30 Hz solid; unlimited alternating dash'}</span><span>{lang === 'cn' ? '连线颜色表示优先级：红色较高，蓝色较低；具体 nice 值见连线详情' : 'Line color shows priority: red is higher, blue is lower; inspect the line for its exact nice value'}</span></div><section className="node-graph-set-library"><h2>{copy.nodeSets}</h2>{nodeSets.length ? <><label>{copy.chooseSet}<select value={setKey} onChange={(event) => { const key = event.target.value; const chosenSet = nodeSets.find((set) => `${set.package.id}@${set.package.version}/${set.id}` === key); setSetKey(key); setSetValues(Object.fromEntries(Object.entries(chosenSet?.parameters || {}).map(([name, parameter]) => [name, parameter.default || '']))); setSetPreview(null); }}><option value="">—</option>{nodeSets.map((set) => { const key = `${set.package.id}@${set.package.version}/${set.id}`; return <option key={key} value={key}>{set.package.id}@{set.package.version} / {set.id}</option>; })}</select></label>{selectedSet && <><p>{selectedSet.nodes.length} {copy.previewCount}</p>{Object.entries(selectedSet.parameters).map(([name, parameter]) => <label key={name}>{name}<input type="text" value={setValues[name] ?? parameter.default ?? ''} onChange={(event) => { setSetValues((current) => ({ ...current, [name]: event.target.value })); setSetPreview(null); }} /></label>)}<button type="button" disabled={readOnly || !graph || busy} onClick={() => void previewPackageSet()}>{copy.previewSet}</button>{setPreview && <div className="node-graph-set-preview" role="status"><strong>{setPreview.result.added_paths.length} {copy.previewCount}</strong><ul>{setPreview.result.added_paths.map((path) => <li key={path}><code>{path}</code></li>)}</ul><button type="button" onClick={addPreviewedSet} disabled={readOnly || busy || setPreview.source !== JSON.stringify(graph)}>{copy.addSet}</button></div>}</>}</> : <p>{copy.noNodeSets}</p>}</section>{graph && <><h2>{copy.robot}</h2><label>{copy.id}<input value={graph.robot.id} onChange={(event) => edit((current) => ({ ...current, robot: { ...current.robot, id: event.target.value } }))} /></label><label>{copy.name}<input value={graph.robot.name} onChange={(event) => edit((current) => ({ ...current, robot: { ...current.robot, name: event.target.value } }))} /></label></>}</aside>
      <div className="node-graph-canvas" ref={canvasRef} onContextMenu={openQuickAdd} onPointerDownCapture={(event) => { if (event.button === 0 && !(event.target as HTMLElement).closest('.node-graph-edge-menu, .node-graph-quick-add')) { setQuickAdd(null); setEdgeMenu(null); } activeGesture.current = true; }} onPointerUpCapture={() => { activeGesture.current = false; }} onPointerCancelCapture={() => { activeGesture.current = false; }} onMouseMove={revealOverview} onTouchMove={revealOverview}>{graph ? <ReactFlow nodes={nodes} edges={edges} nodeTypes={nodeTypes} nodesConnectable={!readOnly} onInit={(instance) => { setFlowInstance(instance); browserLog('debug', 'Graph canvas initialized'); }} onEdgeContextMenu={openEdgeMenu} onEdgeClick={(_, edge) => { setEdgeInputInvalid(false); setSelectedEdge(edge.id); setSideCollapsed(false); setQuickAdd(null); }} onMove={revealOverview} onNodesChange={onNodesChange} onConnect={connect} onNodeClick={(_, node) => { setFrontNode(node.id); setQuickAdd(null); setSelectedEdge(''); setEdgeInputInvalid(false); }} onNodeDragStart={(_, node) => setFrontNode(node.id)} onSelectionStart={() => { activeGesture.current = true; }} onSelectionEnd={() => { activeGesture.current = false; }} selectionOnDrag selectionMode={SelectionMode.Partial} panOnDrag={[1]} deleteKeyCode={null} onPaneClick={() => { setSelected([]); setQuickAdd(null); setEdgeMenu(null); setSelectedEdge(''); setEdgeInputInvalid(false); }} onDoubleClick={(event) => { if ((event.target as HTMLElement).classList.contains('react-flow__pane')) add(true); }} minZoom={0.2} maxZoom={2}><Background color="var(--border)" /><MiniMap className={showOverview ? 'node-graph-minimap--visible' : 'node-graph-minimap--hidden'} pannable zoomable /></ReactFlow> : rawGraph ? <pre className="node-graph-raw" aria-label={lang === 'cn' ? '原始节点图' : 'Raw node graph'}>{JSON.stringify(rawGraph, null, 2)}</pre> : <div className="node-graph-empty"><p>{copy.disconnected}</p><button type="button" onClick={() => void refresh()}>{copy.connect}</button></div>}{graph && !entries.length && <p className="node-graph-empty">{copy.noNodes}</p>}{edgeMenu && <div className="node-graph-edge-menu" role="menu" style={{ left: edgeMenu.left, top: edgeMenu.top }}><button type="button" role="menuitem" onClick={() => { const edge = edges.find((item) => item.id === edgeMenu.id); if (edge) edit((current) => disconnectInput(current, edge.target, String(edge.targetHandle).replace(/^in:/, ''), edge.source, String(edge.sourceHandle).replace(/^out:/, ''))); setEdgeMenu(null); }}>{copy.disconnect}</button></div>}{quickAdd && <div className="node-graph-quick-add" role="dialog" aria-label={copy.add} style={{ left: quickAdd.left, top: quickAdd.top }} onContextMenu={(event) => event.stopPropagation()}><input ref={quickSearchRef} type="search" aria-label={copy.search} placeholder={copy.search} value={quickSearch} onChange={(event) => setQuickSearch(event.target.value)} /><div className="node-graph-quick-add__list">{quickTypes.map((type) => <button type="button" key={type.id} onMouseEnter={() => setQuickDetail(type.id)} onFocus={() => setQuickDetail(type.id)} onClick={() => addAtCursor(type)}><strong>{type.group}</strong><small>{type.core} / {type.layer} · {type.id}</small></button>)}</div>{quickDetail && <div className="node-graph-quick-add__detail" role="note">{(() => { const type = quickTypes.find((item) => item.id === quickDetail); return type ? <><strong>{type.id}</strong><p>{type.core} / {type.layer} / {type.group} · {type.lifecycle}</p><p>{type.capabilities.join(', ') || '—'}</p><small>{type.inputs.length} {lang === 'cn' ? '输入' : 'inputs'} · {type.outputs.length} {lang === 'cn' ? '输出' : 'outputs'} · {type.settings.length} {lang === 'cn' ? '设置' : 'settings'}</small></> : null; })()}</div>}</div>}
        {visible && <><GraphCoreWindow title="EI" state={coreWindows.ei} onChange={(patch) => updateCoreWindow('ei', patch)} lang={lang}><EiWorkbench lang={lang} onOpenGraph={() => updateCoreWindow('ei', { open: false })} /></GraphCoreWindow></>}
      </div>
      <aside className="node-graph-side" hidden={sideCollapsed}>{edgeDetail ? <EdgeIdlPanel key={edgeDetail.id} edge={edgeDetail} entries={entries} dataTypes={dataTypes} diagnostics={diagnostics} validated={validationCurrent} processStatus={processStatus} lang={lang} /> : chosen ? <><fieldset disabled={readOnly} className="node-graph-readonly-fieldset"><NodeInspector key={chosen.path} entry={chosen} entries={entries} lang={lang} options={nodeOptions} runId={processStatus?.run_id || null} activeRevision={processStatus?.active_revision || null} currentRun={!dirty && !readOnly && processStatus?.state === 'running' && processStatus.run_id_state === 'known' && processStatus.active_revision === revision && processStatus.saved_revision === revision} onBind={(target) => { try { edit((current) => bindPrimitiveTarget(current, types, chosen.path, target)); setMessage(''); } catch (error) { setMessage(error instanceof Error ? error.message : String(error), true); } }} onChange={(change) => edit((current) => updateNode(current, chosen.path, change))} onRename={(id) => { try { edit((current) => renameNode(current, chosen.path, id)); setSelected([`${chosen.path.slice(0, chosen.path.lastIndexOf('/') + 1)}${id}`]); setMessage(''); } catch { setMessage(copy.duplicate, true); } }} onRemove={() => { edit((current) => removeNode(current, chosen.path)); setPositions((current) => Object.fromEntries(Object.entries(current).filter(([path]) => path !== chosen.path))); setSelected([]); }} /></fieldset><h3>{copy.target}</h3><ul>{Object.entries(chosen.node.inputs || {}).flatMap(([input, sources]) => inputSources(sources).map((source) => <li key={`${input}:${source.node}:${source.output}`}>{input} ← {canonicalNodePath(source.node)}.{source.output}<button type="button" aria-label={`${copy.remove} ${input}`} disabled={readOnly} onClick={() => edit((current) => disconnectInput(current, chosen.path, input, source.node, source.output))}>×</button></li>))}</ul></> : selected.length > 1 ? <div className="node-graph-multi-selection"><p><strong>{selected.length}</strong> {copy.selectedCount}</p><button type="button" disabled={readOnly} onClick={() => { edit((current) => removeSelectedNodes(current, selected)); setPositions((current) => Object.fromEntries(Object.entries(current).filter(([path]) => !selected.includes(path)))); setSelected([]); }}>{copy.remove}</button></div> : <p>{copy.noSelection}</p>}
        <details className="node-graph-panel" open={diagnosticsOpen} onToggle={(event) => setDiagnosticsOpen(event.currentTarget.open)}>
          <summary><strong>{copy.diagnostics}</strong><span>{diagnostics.length || '✓'}</span></summary>
          {validationCurrent ? diagnostics.length ? <ul>{diagnostics.map((item, index) => <li key={index}><button className="node-graph-diagnostic" type="button" onClick={() => { setSelected([entries.find((entry) => item.path.includes(entry.path))?.path || ''].filter(Boolean)); setSideCollapsed(false); }}><code>{item.path}</code><span>{item.message}</span></button></li>)}</ul> : <p>{copy.valid}</p> : <p>{lang === 'cn' ? '当前草稿尚未检查。' : 'Current draft has not been checked.'}</p>}
        </details>
        <details className="node-graph-panel">
          <summary><strong>{copy.process}</strong><span>{processStatus?.processes.length ?? '—'}</span></summary>
          {processStatus ? <ul>{processStatus.processes.map((process) => <li key={process.id}><strong>{process.id}</strong><span>{process.state || 'running'} · PID {process.pid ?? '—'} · {process.restarts} restarts</span></li>)}</ul> : <p>{copy.noStatus}</p>}
        </details></aside></div>
  </main></ReactFlowProvider>;
}
