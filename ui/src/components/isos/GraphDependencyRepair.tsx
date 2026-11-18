import React, { useState } from 'react';
import { graphPackagePins, installBackend, installSourceBackend, previewPackageDownload, type CoreReadiness, type InstalledPackage, type NodeGraph } from '@/services/isos/node-graph';
import type { Language } from '@/types';

interface Props {
  graph: NodeGraph;
  installed: InstalledPackage[];
  readiness: CoreReadiness | null;
  lang: Language;
  onInstalled: () => void;
  onRetry: () => void;
}

export function GraphDependencyRepair({ graph, installed, readiness, lang, onInstalled, onRetry }: Props): React.ReactElement | null {
  const [sourcePaths, setSourcePaths] = useState<Record<string, string>>({});
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const pins = graphPackagePins(graph);
  const blocked = readiness?.cores.filter((entry) => !entry.ready && entry.reason) || [];
  const repairable = pins.filter((pin) => !installed.some((entry) => entry.id === pin.id && entry.version === pin.version) || blocked.some((entry) => entry.reason?.includes(`${pin.id}@${pin.version}`)));
  if (!repairable.length && !blocked.length) return null;
  const installRelease = async (id: string, version: string): Promise<void> => {
    setBusy(true); setError('');
    try {
      const { preview } = await previewPackageDownload(id, version);
      const prompt = lang === 'cn'
        ? `确认下载 ${preview.id}@${preview.version}？\n发布者：${preview.publisher}\n来源：${preview.source_url || '未提供'}\n地址：${preview.url}\n大小：${preview.size_bytes} 字节\nSHA-256：${preview.sha256}\n信任状态：${preview.trust}`
        : `Approve download of ${preview.id}@${preview.version}?\nPublisher: ${preview.publisher}\nSource: ${preview.source_url || 'unspecified'}\nURL: ${preview.url}\nSize: ${preview.size_bytes} bytes\nSHA-256: ${preview.sha256}\nTrust: ${preview.trust}`;
      if (!window.confirm(prompt)) return;
      await installBackend(id, version, preview.approval);
      onInstalled();
    } catch (cause) { setError(cause instanceof Error ? cause.message : String(cause)); }
    finally { setBusy(false); }
  };
  const installSource = async (id: string, version: string): Promise<void> => {
    const pin = `${id}@${version}`;
    const path = sourcePaths[pin]?.trim() || '';
    if (!path.startsWith('/')) { setError(lang === 'cn' ? '请输入源码检出的绝对路径。' : 'Enter an absolute source checkout path.'); return; }
    if (!window.confirm(lang === 'cn' ? `从本机源码构建并安装 ${pin}？\n${path}` : `Build and install ${pin} from this local source checkout?\n${path}`)) return;
    setBusy(true); setError('');
    try { await installSourceBackend(id, version, path); onInstalled(); }
    catch (cause) { setError(cause instanceof Error ? cause.message : String(cause)); }
    finally { setBusy(false); }
  };
  return <section className="node-graph-dependency-repair" aria-label={lang === 'cn' ? '节点图依赖' : 'Graph dependencies'}>
    <h2>{lang === 'cn' ? '节点图依赖' : 'Graph dependencies'}</h2>
    {blocked.length > 0 && <ul>{blocked.map((entry) => <li key={entry.core}><strong>{entry.core.toUpperCase()}</strong>: {entry.reason}{entry.action ? ` · ${entry.action}` : ''}</li>)}</ul>}
    {repairable.map((pin) => { const key = `${pin.id}@${pin.version}`; const found = installed.some((entry) => entry.id === pin.id && entry.version === pin.version); return <div key={key} className="node-graph-dependency-repair__pin">
      <strong>{key}</strong><p>{found ? (lang === 'cn' ? '已安装，但归属进程仍报告此依赖不可用。' : 'Installed, but an owner still reports this dependency unavailable.') : (lang === 'cn' ? '缺少此精确版本。' : 'This exact package version is missing.')}</p>
      {!found && <button type="button" disabled={busy} onClick={() => void installRelease(pin.id, pin.version)}>{lang === 'cn' ? '安装发行包' : 'Install release'}</button>}
      <label>{lang === 'cn' ? '本机源码检出路径' : 'Local source checkout path'}<input value={sourcePaths[key] || ''} onChange={(event) => setSourcePaths((current) => ({ ...current, [key]: event.target.value }))} placeholder="/absolute/path/to/checkout" /></label>
      <button type="button" disabled={busy || !sourcePaths[key]?.trim()} onClick={() => void installSource(pin.id, pin.version)}>{lang === 'cn' ? '从源码构建并安装（开发模式）' : 'Build and install from source (dev mode)'}</button>
    </div>; })}
    {error && <p role="alert">{error}</p>}
    <button type="button" disabled={busy} onClick={onRetry}>{lang === 'cn' ? '重新检查依赖' : 'Recheck dependencies'}</button>
  </section>;
}
