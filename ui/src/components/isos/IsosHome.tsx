import React, { useCallback, useEffect, useRef, useState } from 'react';
import { getGraphCatalogSnapshot, graphPackagePins, getInstalledPackages, getProcessStatus, getSystemHealth, getSetupStatus, getCoreReadiness, graphCatalogReadOnlyReason, previewBackendLink, previewPackageDownload, installBackend, linkBackend, nodeGraphHasEditableShape, type InstalledPackage, type ProcessStatus, type SystemHealth, type SetupStatus, type CoreReadiness, type LinkPreview } from '@/services/isos/node-graph';
import { IsosLogsPanel } from './IsosLogsPanel';
import { PackageConfigPanel } from './PackageConfigPanel';
import { deploymentStopEpoch, runDeployment, stopDeploymentConverged } from './deployment-control';
import { IsosApiError } from '@/services/isos/api-client';
import { stopReplyFeedback, stopFailureFeedback } from './stop-feedback';
import type { Language } from '@/types';
import { publishUiMessage } from '@/services/isos/ui-messages';
import { startupPhaseText } from './startup-phase';
import { subscribeRuntimeStatus } from '@/services/isos/runtime-status-stream';
import './workbench.scss';

const COPY = {
  en: { title: 'Runtime overview', subtitle: 'Configure registered nodes, verify their packages, then run the graph.', refresh: 'Refresh', packages: 'Packages', installed: 'Installed packages', packageConfiguration: 'Package configuration', configHelp: 'Edit local paths and service settings for an exact installed version. Save, then Apply the graph before Run.', chooseConfig: 'Choose package to configure', noConfig: 'No installed package has a configuration descriptor.', noInstalled: 'No packages installed or linked yet.', linkedSource: 'Local link', archiveSource: 'Installed archive', packageId: 'Package ID', packageVersion: 'Version', manifestPath: 'Release manifest path (optional)', preview: 'Check local bundle', previewReady: 'Bundle verified', previewMissing: 'Enter an absolute bundle directory to check its manifest.', coreReadiness: 'Core build and launch readiness', notConfigured: 'Binary path not configured', filesChanged: 'Configured files changed', configured: 'Configured path', compatible: 'Compatible', graphConfigured: 'Graph configured', running: 'Running', noPackages: 'No package is installed or linked.', required: 'Required by graph', missing: 'This exact package version is missing. An older version does not satisfy the graph.', linked: 'The required version is available.', sceneMismatch: 'The graph scene is not provided by this package.', chooseScene: 'Choose package scene', applyScene: 'Validate and apply scene', sceneApplied: 'Scene saved to the graph. Restart to activate the new revision.', sceneInvalid: 'Graph validation failed. Open Graph for diagnostics.', localBundle: 'Local bundle directory', link: 'Link local bundle', catalog: 'Install from catalog', packageDone: 'Package added. Open Graph to add its nodes.', scenes: 'Scenes', devices: 'Devices', ready: 'Ready', attention: 'Needs attention', setup: 'Setup checks', noIssues: 'All reported checks passed.', processes: 'Managed processes', noProcesses: 'No processes are running.', run: 'Run', stop: 'Stop', restart: 'Restart', logs: 'Logs', saved: 'Saved revision', active: 'Active revision', state: 'State', lastError: 'Last error', graph: 'Edit graph', pending: 'A saved revision differs from the active deployment. Restart to activate it.', unsaved: 'Graph edits are not applied. Apply them on the Graph page before running.', unavailable: 'IsOS is unavailable. Start it locally and refresh.', scopePublish: 'Log scope publication failed before child launch. Status and current logs are being rechecked.', loading: 'Loading…' },
  cn: { title: '运行概览', subtitle: '配置已注册节点并验证软件包，然后运行节点图。', refresh: '刷新', packages: '软件包', installed: '已安装的软件包', packageConfiguration: '软件包配置', configHelp: '编辑指定已安装版本的本机路径和服务设置。保存后，先应用节点图再运行。', chooseConfig: '选择要配置的软件包', noConfig: '已安装软件包均无配置描述。', noInstalled: '尚未安装或链接软件包。', linkedSource: '本地链接', archiveSource: '已安装归档', packageId: '软件包 ID', packageVersion: '版本', manifestPath: '发布清单路径（可选）', preview: '检查本地软件包', previewReady: '软件包已验证', previewMissing: '输入绝对路径以检查软件包清单。', coreReadiness: '核心构建与启动就绪状态', notConfigured: '未配置程序路径', filesChanged: '已配置文件发生变化', configured: '已配置路径', compatible: '兼容', graphConfigured: '节点图配置已生成', running: '运行中', noPackages: '尚未安装或链接软件包。', required: '节点图所需', missing: '缺少此精确版本。旧版本不能满足节点图要求。', linked: '所需版本已就绪。', sceneMismatch: '节点图中的场景不属于此软件包。', chooseScene: '选择软件包场景', applyScene: '验证并应用场景', sceneApplied: '场景已保存到节点图。重启后启用新修订。', sceneInvalid: '节点图验证失败。请前往节点图页面查看诊断。', localBundle: '本地软件包目录', link: '链接本地软件包', catalog: '从目录安装', packageDone: '软件包已添加。请打开节点图添加其节点。', scenes: '场景', devices: '设备', ready: '就绪', attention: '需要处理', setup: '配置检查', noIssues: '已报告的检查均通过。', processes: '受管进程', noProcesses: '没有运行中的进程。', run: '运行', stop: '停止', restart: '重启', logs: '日志', saved: '已保存修订', active: '运行中修订', state: '状态', lastError: '最近错误', graph: '编辑节点图', pending: '已保存修订与运行中修订不同。重启后生效。', unsaved: '节点图有未应用的修改。请先在节点图页面应用，再运行。', unavailable: '无法连接 IsOS。请在本机启动后刷新。', scopePublish: '子进程启动前日志范围发布失败，正在重新检查状态与当前日志。', loading: '加载中…' },
} as const;

interface Props { lang: Language; onOpenGraph: () => void; graphDirty: boolean; onOpenLogs?: () => void; onStopIntent?: (runId: string | null) => void; onStopOutcome?: (feedback: { text: string; alert: boolean }, terminalRun?: string | null) => void }
export function IsosHome({ lang, onOpenGraph, graphDirty, onOpenLogs, onStopIntent, onStopOutcome }: Props): React.ReactElement {
  const copy = COPY[lang];
  const [revision, setRevision] = useState('');
  const [status, setStatus] = useState<ProcessStatus | null>(null);
  const [health, setHealth] = useState<SystemHealth | null>(null);
  const [healthUnavailable, setHealthUnavailable] = useState(false);
  const [installed, setInstalled] = useState<InstalledPackage[]>([]);
  const [configPin, setConfigPin] = useState<{ id: string; version: string } | null>(null);
  const [required, setRequired] = useState<{ id: string; version: string }[]>([]);
  const [readOnlyReason, setReadOnlyReason] = useState<string | null>(null);
  const [bundlePath, setBundlePath] = useState('');
  const [manifestPath, setManifestPath] = useState('');
  const [packageId, setPackageId] = useState('');
  const [packageVersion, setPackageVersion] = useState('');
  const [setup, setSetup] = useState<SetupStatus | null>(null);
  const [readiness, setReadiness] = useState<CoreReadiness | null>(null);
  const [linkPreview, setLinkPreview] = useState<LinkPreview | null>(null);
  const [previewIssue, setPreviewIssue] = useState('');
  const [logsOpen, setLogsOpen] = useState(() => new URLSearchParams(window.location.search).get('logs') === '1');
  const [busy, setBusy] = useState(false);
  const [stopping, setStopping] = useState(false);
  const refreshRequest = useRef(0);
  const statusRequest = useRef(0);
  const healthRequest = useRef(0);
  const statusPollInFlight = useRef(false);
  const healthPollInFlight = useRef(false);
  const streamLive = useRef(false);
  const refresh = useCallback(async (): Promise<void> => {
    const requestId = ++refreshRequest.current;
    const processRequestId = ++statusRequest.current;
    const ownerRequestId = ++healthRequest.current;
    void getSystemHealth().then((next) => {
      if (!streamLive.current && requestId === refreshRequest.current && ownerRequestId === healthRequest.current) { setHealth(next); setHealthUnavailable(false); }
    }).catch(() => { if (!streamLive.current && requestId === refreshRequest.current && ownerRequestId === healthRequest.current) { setHealth(null); setHealthUnavailable(true); } });
    void getProcessStatus().then((process) => {
      if (!streamLive.current && requestId === refreshRequest.current && processRequestId === statusRequest.current) setStatus(process);
    }).catch((error: unknown) => {
      if (requestId === refreshRequest.current) publishUiMessage(error instanceof Error ? error.message : copy.unavailable, 'error');
    });
    void getInstalledPackages().then((receipts) => {
      if (requestId !== refreshRequest.current) return;
      setInstalled(receipts.installed);
      setConfigPin((current) => {
        if (current && receipts.installed.some((entry) => entry.configurable && entry.id === current.id && entry.version === current.version)) return current;
        const configurable = receipts.installed.filter((entry) => entry.configurable);
        return configurable.length === 1 ? { id: configurable[0].id, version: configurable[0].version } : null;
      });
    }).catch(() => undefined);
    void getSetupStatus().then((checks) => { if (requestId === refreshRequest.current) setSetup(checks); }).catch(() => undefined);
    void getCoreReadiness().then((cores) => { if (requestId === refreshRequest.current) setReadiness(cores); }).catch(() => undefined);
    try {
      const snapshot = await getGraphCatalogSnapshot();
      const { graph } = snapshot;
      if (requestId !== refreshRequest.current) return;
      const shaped = nodeGraphHasEditableShape(graph.graph);
      setRevision(graph.revision); setReadOnlyReason(graphCatalogReadOnlyReason(snapshot));
      setRequired(shaped ? graphPackagePins(graph.graph) : []);
    } catch (error) { if (requestId === refreshRequest.current) { setReadOnlyReason('Graph catalog could not be checked.'); publishUiMessage(error instanceof Error ? error.message : copy.unavailable, 'error'); } }
  }, [copy.unavailable]);
  useEffect(() => {
    setLinkPreview(null);
    setPreviewIssue('');
    if (!bundlePath.trim().startsWith('/') || (manifestPath.trim() && !manifestPath.trim().startsWith('/'))) return;
    let cancelled = false;
    const timer = window.setTimeout(() => {
      void previewBackendLink(bundlePath.trim(), manifestPath.trim() || undefined).then((preview) => {
        if (cancelled) return;
        setLinkPreview(preview);
        setPackageId(preview.id);
        setPackageVersion(preview.version);
      }).catch((error: unknown) => { if (!cancelled) setPreviewIssue(error instanceof Error ? error.message : String(error)); });
    }, 350);
    return () => { cancelled = true; window.clearTimeout(timer); };
  }, [bundlePath, manifestPath]);

  useEffect(() => {
    void refresh();
    let active = true;
    const unsubscribe = subscribeRuntimeStatus((snapshot) => {
      if (!active) return;
      streamLive.current = snapshot !== null;
      if (snapshot) {
        ++statusRequest.current;
        ++healthRequest.current;
        setStatus(snapshot.process_status);
        setHealth(snapshot.system_health);
        setHealthUnavailable(false);
      }
    });
    const timer = window.setInterval(() => {
      if (document.visibilityState !== 'visible' || streamLive.current) return;
      if (!statusPollInFlight.current) {
        statusPollInFlight.current = true;
        const requestId = ++statusRequest.current;
        void getProcessStatus().then((next) => { if (active && requestId === statusRequest.current) setStatus(next); })
          .catch(() => { if (active && requestId === statusRequest.current) setStatus(null); })
          .finally(() => { statusPollInFlight.current = false; });
      }
      if (!healthPollInFlight.current) {
        healthPollInFlight.current = true;
        const ownerRequestId = ++healthRequest.current;
        void getSystemHealth().then((next) => { if (active && ownerRequestId === healthRequest.current) { setHealth(next); setHealthUnavailable(false); } })
          .catch(() => { if (active && ownerRequestId === healthRequest.current) { setHealth(null); setHealthUnavailable(true); } })
          .finally(() => { healthPollInFlight.current = false; });
      }
    }, 1500);
    return () => { active = false; streamLive.current = false; unsubscribe(); ++refreshRequest.current; ++statusRequest.current; ++healthRequest.current; window.clearInterval(timer); };
  }, [refresh]);
  const control = async (operation: 'run' | 'stop' | 'restart'): Promise<void> => {
    if (operation === 'stop' && stopping) return;
    if (operation !== 'stop' && (busy || stopping)) return;
    if (operation === 'stop') onStopIntent?.(status?.run_id || null);
    if (operation !== 'stop' && readOnlyReason) return;
    refreshRequest.current += 1;
    statusRequest.current += 1;
    healthRequest.current += 1;
    const stopAtStart = deploymentStopEpoch();
    if (operation === 'stop') {
      setStopping(true);
      const before = status;
      publishUiMessage(lang === 'cn' ? '正在停止…' : 'Stopping…');
      try {
        const reply = await stopDeploymentConverged();
        const after = await getProcessStatus().catch(() => null);
        const feedback = stopReplyFeedback(reply, before, lang, after); publishUiMessage(feedback.text, feedback.alert ? 'error' : 'info'); onStopOutcome?.(feedback, reply.terminal?.run_id || null);
        if (after) setStatus(after);
      } catch (error) { const feedback = stopFailureFeedback(error, lang); publishUiMessage(feedback.text, feedback.alert ? 'error' : 'info'); onStopOutcome?.(feedback); }
      finally { setStopping(false); }
      return;
    }
    setBusy(true);
    if (onOpenLogs) onOpenLogs(); else setLogsOpen(true);
    try {
      if (stopAtStart !== deploymentStopEpoch()) return;
      await runDeployment(operation, revision);
      if (stopAtStart === deploymentStopEpoch()) await refresh();
    }
    catch (error) {
      if (stopAtStart !== deploymentStopEpoch()) return;
      if (error instanceof IsosApiError && error.code === 'log_scope_publish_failed') {
        await refresh();
        window.dispatchEvent(new Event('isosui:log-scope-recheck'));
        publishUiMessage(copy.scopePublish, 'error');
      } else publishUiMessage(error instanceof Error ? error.message : String(error), 'error');
    }
    finally { setBusy(false); }
  };
  const addPackage = async (source: 'link' | 'catalog'): Promise<void> => {
    const id = packageId.trim(); const version = packageVersion.trim();
    const invalidLink = source === 'link' && (!linkPreview?.ready || linkPreview.id !== id || linkPreview.version !== version);
    if (!id || !version || invalidLink) return;
    refreshRequest.current += 1;
    statusRequest.current += 1;
    healthRequest.current += 1;
    setBusy(true);
    try {
      if (source === 'link') await linkBackend(id, version, bundlePath.trim(), manifestPath.trim() || undefined);
      else {
        const { preview } = await previewPackageDownload(id, version);
        const prompt = lang === 'cn'
          ? `确认下载以下软件包？\n${preview.id}@${preview.version}\n发布者：${preview.publisher}\n来源：${preview.source_url || '未提供'}\n地址：${preview.url}\n大小：${preview.size_bytes} 字节\nSHA-256：${preview.sha256}\n信任状态：${preview.trust}`
          : `Approve download of this package?\n${preview.id}@${preview.version}\nPublisher: ${preview.publisher}\nSource: ${preview.source_url || 'unspecified'}\nURL: ${preview.url}\nSize: ${preview.size_bytes} bytes\nSHA-256: ${preview.sha256}\nTrust: ${preview.trust}`;
        if (!window.confirm(prompt)) return;
        await installBackend(id, version, preview.approval);
      }
      await refresh(); window.dispatchEvent(new Event('isosui:packages-changed')); publishUiMessage(copy.packageDone);
    } catch (error) { publishUiMessage(error instanceof Error ? error.message : String(error), 'error'); }
    finally { setBusy(false); }
  };
  const savedRevision = status?.saved_revision || revision;
  const configurablePackages = installed.filter((entry) => entry.configurable);
  const selectedConfig = configPin ? `${configPin.id}@${configPin.version}` : '';
  return <main className="isos-home">
    <header className="isos-home__heading"><div><p className="isos-eyebrow">IsOS</p><h1>{copy.title}</h1><p>{copy.subtitle}</p></div><div className="isos-home__actions"><button type="button" onClick={() => void refresh()} disabled={busy}>{copy.refresh}</button><button type="button" onClick={onOpenGraph}>{copy.graph}</button><button type="button" aria-expanded={onOpenLogs ? undefined : logsOpen} onClick={() => onOpenLogs ? onOpenLogs() : setLogsOpen((open) => !open)}>{copy.logs}</button></div></header>
    {readOnlyReason && <p role="alert" className="isos-home__notice">{lang === 'cn' ? '只读节点图：' : 'Read-only graph: '}{readOnlyReason}</p>}
    <section className="isos-home__overview" aria-label={copy.state}><div><span>{copy.state}</span><strong>{status?.state || (status ? (status.processes.length ? 'running' : 'stopped') : '—')}</strong></div><div><span>{copy.saved}</span><code>{savedRevision || '—'}</code></div><div><span>{copy.active}</span><code>{status?.active_revision || '—'}</code></div></section>
    <section className="isos-home__owners" aria-label={lang === 'cn' ? '归属方服务状态' : 'Owner service status'}><h2>{lang === 'cn' ? '归属方服务' : 'Owner services'}</h2><div>{(['eos', 'ei'] as const).map((core) => { const owner = health?.owners.find((entry) => entry.core === core); const mismatch = !!owner?.run_id && !!status && (owner.run_id !== status.run_id || owner.revision !== status.active_revision); return <article key={core}><strong>{core.toUpperCase()}</strong><span role={healthUnavailable || owner?.state === 'failed' || owner?.state === 'degraded' || owner?.state === 'unavailable' || mismatch ? 'alert' : 'status'}>{owner?.state || (healthUnavailable ? (lang === 'cn' ? '不可用' : 'Unavailable') : (lang === 'cn' ? '检查中…' : 'Checking…'))}</span>{owner?.reason && <small>{owner.reason}</small>}{mismatch && <small role="alert">{lang === 'cn' ? '归属方运行身份与 IsOS 当前运行不一致' : 'Owner run identity differs from the selected IsOS run'}</small>}{owner?.run_id && <code>{owner.run_id}</code>}</article>; })}</div><p>{lang === 'cn' ? '服务健康状态仅表示归属方可访问；运行就绪和 Stop 确认以部署状态为准。' : 'Service health shows owner reachability. Deployment status determines run readiness and Stop confirmation.'}</p></section>
    {startupPhaseText(status, lang, busy) && <p role="status" className="isos-home__notice">{startupPhaseText(status, lang, busy)} · <button type="button" onClick={() => onOpenLogs ? onOpenLogs() : setLogsOpen(true)}>{copy.logs}</button></p>}
    {status?.active_revision && status.active_revision !== savedRevision && <p role="status" className="isos-home__notice">{copy.pending}</p>}
    {graphDirty && <p role="status" className="isos-home__notice">{copy.unsaved}</p>}
    <section className="isos-home__controls" aria-label={copy.processes}><button type="button" onClick={() => void control('run')} disabled={!!readOnlyReason || busy || stopping || !revision || graphDirty}>{copy.run}</button><button type="button" onClick={() => void control('restart')} disabled={!!readOnlyReason || busy || stopping || !revision || graphDirty}>{copy.restart}</button><button type="button" onClick={() => void control('stop')} disabled={stopping}>{copy.stop}</button></section>
    <section className="isos-home__configuration" aria-label={copy.packageConfiguration}>
      <h2>{copy.packageConfiguration}</h2><p>{copy.configHelp}</p>
      {configurablePackages.length ? <>
        <label htmlFor="isos-package-config-select">{copy.chooseConfig}</label>
        <select id="isos-package-config-select" value={selectedConfig} onChange={(event) => {
          const next = configurablePackages.find((entry) => `${entry.id}@${entry.version}` === event.target.value);
          setConfigPin(next ? { id: next.id, version: next.version } : null);
        }}><option value="">—</option>{configurablePackages.map((entry) => <option key={`${entry.id}@${entry.version}`} value={`${entry.id}@${entry.version}`}>{entry.id}@{entry.version}</option>)}</select>
        {configPin && <PackageConfigPanel key={selectedConfig} id={configPin.id} version={configPin.version} lang={lang} onSaved={() => window.dispatchEvent(new Event('isosui:packages-changed'))} />}
      </> : <p>{copy.noConfig}</p>}
    </section>
    <div className="isos-home__columns"><section><h2>{copy.installed}</h2>{installed.length ? <ul className="isos-home__installed">{installed.map((entry) => <li key={`${entry.id}@${entry.version}`}><strong>{entry.id}</strong><code>{entry.version}</code><span>{entry.local_link ? copy.linkedSource : copy.archiveSource}</span></li>)}</ul> : <p>{copy.noInstalled}</p>}<h2>{copy.packages}</h2>{required.map((pin) => { const found = installed.some((entry) => entry.id === pin.id && entry.version === pin.version); return <div className="isos-home__package-required" key={`${pin.id}@${pin.version}`}><strong>{copy.required}: <code>{pin.id}@{pin.version}</code></strong><p role="status" className={found ? 'isos-ready' : 'isos-attention'}>{found ? copy.linked : copy.missing}</p></div>; })}<div className="isos-home__package-install"><label htmlFor="isos-package-id">{copy.packageId}</label><input id="isos-package-id" value={packageId} onChange={(event) => setPackageId(event.target.value)} placeholder="example/driver" autoComplete="off" /><label htmlFor="isos-package-version">{copy.packageVersion}</label><input id="isos-package-version" value={packageVersion} onChange={(event) => setPackageVersion(event.target.value)} placeholder="2.0.0" autoComplete="off" /><label htmlFor="isos-bundle-path">{copy.localBundle}</label><input id="isos-bundle-path" value={bundlePath} onChange={(event) => setBundlePath(event.target.value)} placeholder="/absolute/path/to/bundle" autoComplete="off" /><label htmlFor="isos-manifest-path">{copy.manifestPath}</label><input id="isos-manifest-path" value={manifestPath} onChange={(event) => setManifestPath(event.target.value)} placeholder="/absolute/path/to/release.json" autoComplete="off" />{bundlePath.trim() && <div className="isos-home__link-preview" role={previewIssue || linkPreview && !linkPreview.ready ? 'alert' : 'status'}><strong>{copy.preview}</strong>{previewIssue ? <p>{previewIssue}</p> : linkPreview ? <><p>{linkPreview.id}@{linkPreview.version} · {linkPreview.ready ? copy.previewReady : copy.attention}</p>{linkPreview.issues.length > 0 && <ul>{linkPreview.issues.map((issue, index) => <li key={index}>{issue}</li>)}</ul>}</> : <p>{copy.loading}</p>}</div>}<div><button type="button" disabled={busy || !linkPreview?.ready || linkPreview.id !== packageId.trim() || linkPreview.version !== packageVersion.trim()} onClick={() => void addPackage('link')}>{copy.link}</button><button type="button" disabled={busy || !packageId.trim() || !packageVersion.trim()} onClick={() => void addPackage('catalog')}>{copy.catalog}</button></div></div></section>
    <section><h2>{copy.coreReadiness}</h2>{readiness?.cores.length ? <ul className="isos-home__readiness">{readiness.cores.map((core) => <li key={core.core}><strong>{core.core.toUpperCase()} · {core.version || '—'}</strong><p>{copy.configured}: <code>{core.path || copy.notConfigured}</code></p><p>{copy.compatible}: {core.compatible ? copy.ready : copy.attention} · {copy.graphConfigured}: {core.graph_configured ? copy.ready : copy.attention} · {copy.running}: {core.running ? copy.ready : '—'}</p>{core.reason && <p role={core.ready ? 'status' : 'alert'}>{core.reason}</p>}{core.action && <p>{core.action}</p>}</li>)}</ul> : <p>{readiness ? copy.noIssues : '—'}</p>}<h2>{copy.setup}</h2>{setup?.issues?.length ? <ul>{setup.issues.map((issue, index) => <li key={index}>{issue}</li>)}</ul> : <p>{setup ? copy.noIssues : '—'}</p>}{status?.last_error && <p role="alert">{copy.lastError}: {status.last_error}</p>}</section></div>
    <section className="isos-home__processes"><h2>{copy.processes}</h2>{status?.processes.length ? <ul>{status.processes.map((process) => <li key={process.id}><div><strong>{process.id}</strong><span>{process.state || (process.pid ? 'running' : 'stopped')} · PID {process.pid ?? '—'} · restarts {process.restarts}</span>{process.last_error && <small role="alert">{process.last_error}</small>}</div></li>)}</ul> : <p>{copy.noProcesses}</p>}</section>
    {logsOpen && !onOpenLogs && <div className="isos-home__logs"><IsosLogsPanel lang={lang} sources={status?.processes.map((process) => process.id)} onClose={() => setLogsOpen(false)} /></div>}
  </main>;
}
