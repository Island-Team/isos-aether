#!/usr/bin/env node
import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { render, Box, Text, useApp, useInput } from 'ink';
import { createIsosClient, isLoopbackOrigin } from '../services/isos/transport';
import type { CoreReadiness, CurrentLogPage, ProcessStatus, SetupStatus } from '../services/isos/node-graph';

type Locale = 'en' | 'cn';
type View = 'home' | 'eos' | 'ei' | 'logs';
interface UiConfig { default_interface: 'web' | 'tui'; locale: Locale; api_origin: string; poll_interval_ms: number }
const words = {
  en: { title: 'IsOS', home: 'Home', eos: 'EOS', ei: 'EI', logs: 'Logs', state: 'State', saved: 'Saved revision', active: 'Active revision', run: 'Run', stop: 'Stop', refresh: 'Refresh', quit: 'Quit', loading: 'Connecting to IsOS…', unavailable: 'Unavailable', noLogs: 'No current events', noCore: 'No owner state', ready: 'Ready', blocked: 'Blocked', confirmRun: 'Press r again to Run the saved revision', confirmStop: 'Press s again to Stop the deployment', cancelled: 'Action cancelled', pending: 'Working…', unknown: 'Stop outcome is uncertain; inspect IsOS status and logs.' },
  cn: { title: 'IsOS', home: '首页', eos: 'EOS', ei: 'EI', logs: '日志', state: '状态', saved: '已保存修订', active: '活动修订', run: '运行', stop: '停止', refresh: '刷新', quit: '退出', loading: '正在连接 IsOS…', unavailable: '不可用', noLogs: '当前没有事件', noCore: '没有所有者状态', ready: '就绪', blocked: '阻止', confirmRun: '再次按 r 运行已保存修订', confirmStop: '再次按 s 停止部署', cancelled: '已取消操作', pending: '处理中…', unknown: '停止结果不确定；请查看 IsOS 状态和日志。' },
};

export function parseArgs(args: string[]): { origin: string; locale: Locale } {
  let origin = 'http://127.0.0.1:16000';
  let locale: Locale = 'en';
  for (let index = 0; index < args.length; index += 1) {
    if (args[index] === '--origin' && args[index + 1]) origin = args[++index];
    else if (args[index] === '--locale' && args[index + 1]) {
      const value = args[++index];
      if (value !== 'en' && value !== 'cn') throw new Error('--locale must be en or cn');
      locale = value;
    } else throw new Error(`Unknown TUI argument: ${args[index]}`);
  }
  if (!isLoopbackOrigin(origin)) throw new Error('--origin must be http://127.0.0.1:<port>');
  return { origin, locale };
}

function App({ origin, initialLocale }: { origin: string; initialLocale: Locale }): React.ReactElement {
  const client = useMemo(() => createIsosClient(origin), [origin]);
  const { exit } = useApp();
  const [locale, setLocale] = useState(initialLocale);
  const [pollInterval, setPollInterval] = useState(2000);
  const [view, setView] = useState<View>('home');
  const [status, setStatus] = useState<ProcessStatus | null>(null);
  const [readiness, setReadiness] = useState<CoreReadiness | null>(null);
  const [setup, setSetup] = useState<SetupStatus | null>(null);
  const [logs, setLogs] = useState<CurrentLogPage | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [confirm, setConfirm] = useState<'run' | 'stop' | null>(null);
  const [busy, setBusy] = useState(false);
  const copy = words[locale];

  const refresh = useCallback(async (): Promise<void> => {
    const results = await Promise.allSettled([
      client<ProcessStatus>('/runtime/process-status'),
      client<CoreReadiness>('/cores/readiness'),
      client<SetupStatus>('/cores/setup-status'),
      client<CurrentLogPage>('/runtime/logs?view=current&source=all&level=info&limit=30'),
    ]);
    if (results[0].status === 'fulfilled') { setStatus(results[0].value); setError(null); }
    else setError(String(results[0].reason));
    if (results[1].status === 'fulfilled') setReadiness(results[1].value);
    if (results[2].status === 'fulfilled') setSetup(results[2].value);
    if (results[3].status === 'fulfilled') setLogs(results[3].value);
  }, [client]);

  useEffect(() => {
    void client<UiConfig>('/system/ui-config').then((config) => {
      if (config.locale === 'en' || config.locale === 'cn') setLocale(config.locale);
      if (Number.isInteger(config.poll_interval_ms) && config.poll_interval_ms >= 500 && config.poll_interval_ms <= 10000) setPollInterval(config.poll_interval_ms);
    }).catch(() => { /* Older IsOS releases can still display the TUI. */ });
    void refresh();
    const timer = setInterval(() => void refresh(), pollInterval);
    return () => clearInterval(timer);
  }, [client, refresh, pollInterval]);

  const control = async (operation: 'run' | 'stop'): Promise<void> => {
    setBusy(true);
    setConfirm(null);
    setNotice(copy.pending);
    try {
      const reply = await client<unknown>('/runtime/control', 'POST', {
        schema_version: '0.1.0', operation,
        ...(operation === 'run' && status?.saved_revision ? { revision: status.saved_revision } : {}),
      });
      if (operation === 'stop' && reply && typeof reply === 'object' && 'terminal' in reply &&
          reply.terminal && typeof reply.terminal === 'object' && 'outcome' in reply.terminal && reply.terminal.outcome === 'unknown') {
        setNotice(copy.unknown);
      } else setNotice(`${operation === 'run' ? copy.run : copy.stop}: ${copy.ready}`);
      await refresh();
    } catch (cause) { setNotice(cause instanceof Error ? cause.message : String(cause)); }
    finally { setBusy(false); }
  };

  useInput((input) => {
    if (input === 'q') { exit(); return; }
    if (input >= '1' && input <= '4') { setView((['home', 'eos', 'ei', 'logs'] as View[])[Number(input) - 1]); setConfirm(null); return; }
    if (input === 'x') { setConfirm(null); setNotice(copy.cancelled); return; }
    if (input === 'f') { void refresh(); return; }
    if (busy || (input !== 'r' && input !== 's')) return;
    const operation = input === 'r' ? 'run' : 'stop';
    if (confirm !== operation) { setConfirm(operation); setNotice(operation === 'run' ? copy.confirmRun : copy.confirmStop); return; }
    void control(operation);
  });

  const owner = readiness?.cores.filter((entry) => entry.core.toLowerCase() === view);
  return <Box flexDirection="column" padding={1}>
    <Text bold color="cyan">{copy.title}  1 {copy.home}  2 {copy.eos}  3 {copy.ei}  4 {copy.logs}</Text>
    <Text dimColor>r {copy.run}  s {copy.stop}  f {copy.refresh}  x cancel  q {copy.quit}</Text>
    <Box marginTop={1} flexDirection="column">
      {view === 'home' && <>
        <Text>{copy.state}: {status?.state ?? copy.unavailable}</Text>
        <Text>{copy.saved}: {status?.saved_revision ?? '—'}</Text>
        <Text>{copy.active}: {status?.active_revision ?? '—'}</Text>
        <Text>{copy.ready}: {setup?.ready ? 'yes' : copy.blocked}</Text>
        {setup?.issues?.map((issue, index) => <Text key={index} color="yellow">• {issue}</Text>)}
      </>}
      {(view === 'eos' || view === 'ei') && (owner?.length ? owner.map((entry, index) => <Text key={index} color={entry.ready ? 'green' : 'yellow'}>{entry.path ?? entry.core}: {entry.ready ? copy.ready : entry.reason ?? copy.blocked}</Text>) : <Text>{copy.noCore}</Text>)}
      {view === 'logs' && (logs?.lines.length ? logs.lines.slice(-20).map((line, index) => <Text key={index}>[{line.level}] {line.source}: {line.message}</Text>) : <Text>{copy.noLogs}</Text>)}
    </Box>
    {error && <Text color="red">{error}</Text>}
    {notice && <Text color="yellow">{notice}</Text>}
    {!status && !error && <Text>{copy.loading}</Text>}
  </Box>;
}

if (process.argv[1] && import.meta.url === new URL(`file://${process.argv[1]}`).href) {
  try {
    const { origin, locale } = parseArgs(process.argv.slice(2));
    render(<App origin={origin} initialLocale={locale} />);
  } catch (error) {
    process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`);
    process.exitCode = 2;
  }
}
