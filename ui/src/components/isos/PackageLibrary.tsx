import React, { useState } from 'react';
import type { Language } from '@/types';
import { catalogTypeKey, type InstalledPackage, type NodeTypeDefinition } from '@/services/isos/node-graph';
import { packageAccent, packagePin, TypeTree } from './CatalogBrowser';

export const installedPin = (entry: Pick<InstalledPackage, 'id' | 'version'>): string => `${entry.id}@${entry.version}`;

export function PackageLibrary({ installed, types, selectedPin, selectedType, lang, onSelectPackage, onSelectType, onRefresh }: { installed: InstalledPackage[]; types: NodeTypeDefinition[]; selectedPin: string; selectedType: string; lang: Language; onSelectPackage: (pin: string) => boolean; onSelectType: (key: string) => void; onRefresh: () => void }): React.ReactElement {
  const [search, setSearch] = useState('');
  const copy = lang === 'cn' ? { search: '搜索软件包', refresh: '刷新软件包', empty: '没有已安装的软件包。', noMatch: '没有匹配的软件包。' } : { search: 'Search packages', refresh: 'Refresh packages', empty: 'No installed packages.', noMatch: 'No matching packages.' };
  const visible = [...installed].sort((a, b) => installedPin(a).localeCompare(installedPin(b))).filter((entry) => installedPin(entry).toLowerCase().includes(search.trim().toLowerCase()));
  return <div className="node-package-library">
    <div className="node-package-library__header"><label>{copy.search}<input type="search" value={search} onChange={(event) => setSearch(event.target.value)} /></label><button type="button" onClick={onRefresh} aria-label={copy.refresh} title={copy.refresh}>↻</button></div>
    {!visible.length && <p>{installed.length ? copy.noMatch : copy.empty}</p>}
    {visible.map((entry) => {
      const pin = installedPin(entry);
      const ownTypes = types.filter((type) => packagePin(type) === pin && type.implementation?.kind !== 'managed_process');
      return <details key={pin} className="node-package-library__entry" open={search ? true : undefined} style={{ '--package-accent': packageAccent(pin) } as React.CSSProperties}>
        <summary aria-current={selectedPin === pin ? 'true' : undefined} onClick={() => onSelectPackage(pin)}><span className="node-library-package-dot" aria-hidden="true" />{pin}</summary>
        <TypeTree types={ownTypes} value={selectedType} lang={lang} onChange={(key) => { if (onSelectPackage(pin)) onSelectType(key); }} packageScoped />
      </details>;
    })}
  </div>;
}
