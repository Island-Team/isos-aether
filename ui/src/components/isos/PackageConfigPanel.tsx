import React, { useEffect, useState } from 'react';
import { getPackageConfig, putPackageConfig, validatePackageConfig, type PackageConfigDocument, type PackageConfigField } from '@/services/isos/node-graph';
import type { Language } from '@/types';

type Values = Record<string, unknown>;

function sectionValues(values: Values, path: string): Values {
  let current: unknown = values;
  for (const part of path.split('.')) {
    if (!current || typeof current !== 'object' || Array.isArray(current)) return {};
    current = (current as Values)[part];
  }
  return current && typeof current === 'object' && !Array.isArray(current) ? current as Values : {};
}

function withField(values: Values, path: string, field: string, value: unknown): Values {
  const next = structuredClone(values);
  let section: Values = next;
  for (const part of path.split('.')) {
    const current = section[part];
    if (!current || typeof current !== 'object' || Array.isArray(current)) section[part] = {};
    section = section[part] as Values;
  }
  if (value === undefined) delete section[field];
  else section[field] = value;
  return next;
}

function fieldValue(field: PackageConfigField, current: unknown): string {
  const value = current ?? field.default;
  return value === undefined ? '' : String(value);
}

function parsedFieldValue(field: PackageConfigField, raw: string): unknown {
  if (!raw && field.type !== 'boolean') return undefined;
  if (field.type === 'integer') return raw === '' ? undefined : Number(raw);
  if (field.type === 'boolean') return raw === '' ? undefined : raw === 'true';
  return raw;
}

interface Props { id: string; version: string; lang: Language; onSaved?: () => void; onDirtyChange?: (dirty: boolean) => void }
export function PackageConfigPanel({ id, version, lang, onSaved, onDirtyChange }: Props): React.ReactElement {
  const [document, setDocument] = useState<PackageConfigDocument | null>(null);
  const [values, setValues] = useState<Values>({});
  const [toml, setToml] = useState('');
  const [mode, setMode] = useState<'form' | 'toml'>('form');
  const [pending, setPending] = useState(false);
  const [message, setMessage] = useState('');
  const [diagnostics, setDiagnostics] = useState<string[]>([]);
  const [requestId, setRequestId] = useState(0);
  const copy = lang === 'cn'
    ? { title: '软件包配置', form: '表单', raw: 'TOML', validate: '检查配置', save: '保存配置', reload: '重新加载', discard: '当前配置有未保存的修改。继续将丢弃这些修改。', loading: '正在读取配置…', missing: '此软件包没有可用的配置描述。', saved: '配置已保存。重新应用节点图后生效。', valid: '配置有效。', required: '必填', revision: '配置修订', path: '本机路径；IsOS 不会自动下载。' }
    : { title: 'Package configuration', form: 'Form', raw: 'TOML', validate: 'Validate', save: 'Save configuration', reload: 'Reload', discard: 'You have unsaved configuration changes. Continuing will discard them.', loading: 'Loading configuration…', missing: 'No configuration descriptor is available for this package.', saved: 'Configuration saved. Apply the graph to use it.', valid: 'Configuration is valid.', required: 'required', revision: 'Configuration revision', path: 'Local path; IsOS does not download it.' };
  useEffect(() => {
    let active = true;
    setDocument(null);
    setMessage('');
    setDiagnostics([]);
    void getPackageConfig(id, version).then((next) => {
      if (!active) return;
      setDocument(next);
      setValues(next.values || {});
      setToml(next.toml);
      setMode('form');
    }).catch((error: unknown) => { if (active) setMessage(error instanceof Error ? error.message : copy.missing); });
    return () => { active = false; };
  }, [id, version, requestId]);
  const update = mode === 'form' ? { values } : { toml };
  const formDirty = document !== null && JSON.stringify(values) !== JSON.stringify(document.values || {});
  const tomlDirty = document !== null && toml !== document.toml;
  useEffect(() => { onDirtyChange?.(mode === 'form' ? formDirty : tomlDirty); }, [mode, formDirty, tomlDirty, onDirtyChange]);
  const changeMode = (next: 'form' | 'toml'): void => {
    if (next === mode || !document) return;
    if ((mode === 'form' ? formDirty : tomlDirty) && !window.confirm(copy.discard)) return;
    setValues(document.values || {});
    setToml(document.toml);
    setMode(next);
    setDiagnostics([]);
    setMessage('');
  };
  const reload = (): void => {
    if ((formDirty || tomlDirty) && !window.confirm(copy.discard)) return;
    setRequestId((value) => value + 1);
  };
  const check = async (): Promise<boolean> => {
    setPending(true);
    setDiagnostics([]);
    setMessage('');
    try {
      const result = await validatePackageConfig(id, version, update);
      setDiagnostics(result.diagnostics.map((item) => item.path ? `${item.path}: ${item.message}` : item.message));
      if (result.valid) setMessage(copy.valid);
      return result.valid;
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
      return false;
    } finally { setPending(false); }
  };
  const save = async (): Promise<void> => {
    setPending(true);
    setDiagnostics([]);
    setMessage('');
    try {
      const result = await validatePackageConfig(id, version, update);
      if (!result.valid) {
        setDiagnostics(result.diagnostics.map((item) => item.path ? `${item.path}: ${item.message}` : item.message));
        return;
      }
      await putPackageConfig(id, version, document?.revision || '', update);
      setMessage(copy.saved);
      setRequestId((value) => value + 1);
      onSaved?.();
    } catch (error) { setMessage(error instanceof Error ? error.message : String(error)); }
    finally { setPending(false); }
  };
  return <section className="isos-package-config" aria-label={copy.title}>
    <header><h3>{copy.title}: <code>{id}@{version}</code></h3><button type="button" onClick={reload} disabled={pending}>{copy.reload}</button></header>
    {!document ? <p role={message ? 'alert' : 'status'}>{message || copy.loading}</p> : <>
      <p><small>{copy.revision}: <code>{document.revision || '—'}</code></small></p>
      <div className="isos-package-config__tabs" role="group" aria-label={copy.title}>
        <button type="button" aria-pressed={mode === 'form'} onClick={() => changeMode('form')}>{copy.form}</button>
        <button type="button" aria-pressed={mode === 'toml'} onClick={() => changeMode('toml')}>{copy.raw}</button>
      </div>
      {mode === 'form' ? document.descriptor.sections.map((section) => <fieldset key={section.id}>
        <legend>{section.label[lang]}</legend>
        <p>{section.description[lang]}</p>
        {section.fields.map((field) => {
          const current = sectionValues(values, section.id)[field.name];
          const fieldId = `pkg-config-${section.id}-${field.name}`.replace(/[^A-Za-z0-9_-]/g, '-');
          return <label key={field.name} htmlFor={fieldId}>
            <span>{field.label[lang]}{field.required ? ` · ${copy.required}` : ''}</span>
            {field.type === 'enum' ? <select id={fieldId} value={fieldValue(field, current)} onChange={(event) => setValues((old) => withField(old, section.id, field.name, parsedFieldValue(field, event.target.value)))}><option value="">—</option>{(field.options || []).map((option) => <option key={option.value} value={option.value}>{option.label[lang]}</option>)}</select>
              : field.type === 'boolean' ? <select id={fieldId} value={fieldValue(field, current)} onChange={(event) => setValues((old) => withField(old, section.id, field.name, parsedFieldValue(field, event.target.value)))}><option value="">—</option><option value="true">true</option><option value="false">false</option></select>
              : <input id={fieldId} type={field.type === 'integer' ? 'number' : 'text'} step={field.type === 'integer' ? 1 : undefined} min={field.min} max={field.max} value={fieldValue(field, current)} onChange={(event) => setValues((old) => withField(old, section.id, field.name, parsedFieldValue(field, event.target.value)))} autoComplete="off" />}
            <small>{field.description[lang]}{field.note ? ` · ${field.note[lang]}` : ''}{field.type === 'directory' || field.type === 'file' ? ` · ${copy.path}` : ''}</small>
          </label>;
        })}
      </fieldset>) : <label className="isos-package-config__raw">{copy.raw}<textarea rows={18} spellCheck={false} value={toml} onChange={(event) => setToml(event.target.value)} /></label>}
      <div className="isos-package-config__actions"><button type="button" onClick={() => void check()} disabled={pending}>{copy.validate}</button><button type="button" onClick={() => void save()} disabled={pending}>{copy.save}</button></div>
      {message && <p role="status">{message}</p>}
      {diagnostics.length > 0 && <ul role="alert">{diagnostics.map((issue, index) => <li key={index}>{issue}</li>)}</ul>}
    </>}
  </section>;
}
