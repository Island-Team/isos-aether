import React, { useMemo, useState } from 'react';
import type { Language } from '@/types';
import { catalogTypeKey, type NodeTypeDefinition } from '../../services/isos/node-graph';

export const packagePin = (type: NodeTypeDefinition): string => type.package ? `${type.package.id}@${type.package.version}` : '';
const accents = ['#6c9bf2', '#bd83de', '#e69b57', '#4db8aa', '#d47891', '#9aa94e', '#729bd2', '#c38c72'];
export function packageAccent(pin: string): string {
  let hash = 0;
  for (const char of pin) hash = (Math.imul(hash, 31) + char.charCodeAt(0)) >>> 0;
  return accents[hash % accents.length];
}

const typeName = (type: NodeTypeDefinition): string => type.id.split('@')[0].split('.').slice(3).join('.') || type.id;
const sortTypes = (types: NodeTypeDefinition[]): NodeTypeDefinition[] => [...types].sort((a, b) => a.id.localeCompare(b.id) || packagePin(a).localeCompare(packagePin(b)));
const unique = (values: string[]): string[] => [...new Set(values)].sort();

export function TypeTree({ types, value, lang, onChange, search = '', packageScoped = false }: { types: NodeTypeDefinition[]; value: string; lang: Language; onChange: (id: string) => void; search?: string; packageScoped?: boolean }): React.ReactElement {
  const query = search.trim().toLowerCase();
  const visible = types.filter((type) => !query || `${type.id} ${type.description || ''} ${type.core} ${type.layer} ${type.group} ${packagePin(type)}`.toLowerCase().includes(query));
  const copy = lang === 'cn' ? { builtIn: '内置', noTypes: '没有匹配的节点类型。', noPackageTypes: '此软件包没有节点类型。' } : { builtIn: 'Built-in', noTypes: 'No matching node types.', noPackageTypes: 'This package contributes no node types.' };
  const typeButton = (type: NodeTypeDefinition): React.ReactElement => <button type="button" key={catalogTypeKey(type)} className="node-library-type" aria-pressed={catalogTypeKey(type) === value} style={type.package ? { '--package-accent': packageAccent(packagePin(type)) } as React.CSSProperties : undefined} onClick={() => onChange(catalogTypeKey(type))}>
    <span className={`node-library-type__mark${type.package ? ' node-library-type__mark--package' : ''}`} aria-hidden="true" />
    <span><strong>{typeName(type)}</strong><small>{type.package ? packagePin(type) : copy.builtIn}</small></span>
  </button>;
  if (!visible.length) return <p className="node-library-empty">{packageScoped ? copy.noPackageTypes : copy.noTypes}</p>;
  return <div className="node-library-tree">
    {unique(visible.map((type) => type.core)).map((core) => <details key={core} open={query ? true : undefined} className="node-library-folder"><summary>{core}</summary>
      {unique(visible.filter((type) => type.core === core).map((type) => type.layer)).map((layer) => <details key={layer} open={query ? true : undefined} className="node-library-folder"><summary>{layer}</summary>
        {unique(visible.filter((type) => type.core === core && type.layer === layer).map((type) => type.group)).map((group) => {
          const groupTypes = sortTypes(visible.filter((type) => type.core === core && type.layer === layer && type.group === group));
          const pins = unique(groupTypes.filter((type) => type.package).map(packagePin));
          return <details key={group} open={query ? true : undefined} className="node-library-folder"><summary>{group}</summary>
            {groupTypes.filter((type) => !type.package || packageScoped || pins.length <= 1).map(typeButton)}
            {!packageScoped && pins.length > 1 && pins.map((pin) => <details key={pin} open={query ? true : undefined} className="node-library-folder node-library-folder--package" style={{ '--package-accent': packageAccent(pin) } as React.CSSProperties}><summary><span className="node-library-package-dot" aria-hidden="true" />{pin}</summary>{groupTypes.filter((type) => packagePin(type) === pin).map(typeButton)}</details>)}
          </details>;
        })}
      </details>)}
    </details>)}
  </div>;
}

export function CatalogBrowser({ types, value, lang, onChange }: { types: NodeTypeDefinition[]; value: string; lang: Language; onChange: (id: string) => void }): React.ReactElement {
  const [search, setSearch] = useState('');
  const browsable = useMemo(() => types.filter((type) => !(type.package && type.implementation?.kind === 'managed_process')), [types]);
  const copy = lang === 'cn' ? { search: '搜索节点类型', description: '类型描述', source: '来源', fallback: '当前目录没有类型描述。', builtIn: '内置' } : { search: 'Search node types', description: 'Type description', source: 'Source', fallback: 'No type description in this catalog.', builtIn: 'Built-in' };
  const selected = browsable.find((type) => catalogTypeKey(type) === value);
  return <div className="node-graph-catalog-browser">
    <label>{copy.search}<input type="search" value={search} onChange={(event) => setSearch(event.target.value)} /></label>
    <TypeTree key={search ? 'search' : 'browse'} types={browsable} value={value} lang={lang} onChange={onChange} search={search} />
    {selected && <p><strong>{selected.id}</strong><br />{selected.core} / {selected.layer} / {selected.group}<br /><strong>{copy.source}:</strong> {packagePin(selected) || copy.builtIn}<br /><strong>{copy.description}:</strong> {selected.description || copy.fallback}</p>}
  </div>;
}
