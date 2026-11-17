import type { ProcessStatus } from '../../services/isos/node-graph';
import type { Language } from '../../types';

const PHASES = {
  building_source: { en: 'Building linked source package', cn: '正在构建本机源码软件包' },
  validating_graph: { en: 'Validating graph', cn: '正在验证节点图' },
  starting_owners: { en: 'Starting EOS and EI', cn: '正在启动 EOS 和 EI' },
} as const;

export function startupPhaseText(status: ProcessStatus | null, lang: Language, pending = false): string | null {
  if (status?.state !== 'starting' && status?.state !== 'stopping' && !pending) return null;
  if (status?.state === 'stopping') return lang === 'cn' ? '正在停止启动…' : 'Stopping startup…';
  const phase = status?.startup_phase;
  if (phase && phase in PHASES) return PHASES[phase as keyof typeof PHASES][lang];
  if (phase) return `${lang === 'cn' ? '启动阶段' : 'Startup phase'}: ${phase.replace(/_/g, ' ')}`;
  return lang === 'cn' ? '正在启动部署；请查看日志了解进度。' : 'Starting deployment; check Logs for progress.';
}
