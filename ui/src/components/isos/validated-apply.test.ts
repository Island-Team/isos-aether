import { describe, expect, it, vi } from 'vitest';
import { applyNodeGraph, putGraphPoolEntry, validateNodeGraph, type NodeGraph } from '@/services/isos/node-graph';
import { validatedApply } from './validated-apply';

vi.mock('@/services/isos/node-graph', () => ({ validateNodeGraph: vi.fn(), putGraphPoolEntry: vi.fn(), applyNodeGraph: vi.fn() }));
const graph: NodeGraph = { version: '0.2.0', robot: { id: 'r', name: 'r' }, cores: {} };

describe('IsOS Validate before Apply', () => {
  it('does not PUT a rejected graph', async () => {
    vi.mocked(applyNodeGraph).mockClear();
    vi.mocked(validateNodeGraph).mockResolvedValue({ valid: false, diagnostics: [{ path: '/inputs/frame', code: 'edge_priority', message: 'Invalid nice' }] });
    const seen = vi.fn();
    expect((await validatedApply(graph, 'demo', 'r1', () => true, seen)).state).toBe('rejected');
    expect(seen).toHaveBeenCalledOnce();
    expect(applyNodeGraph).not.toHaveBeenCalled();
  });
  it('does not PUT when the draft changes during validation', async () => {
    vi.mocked(applyNodeGraph).mockClear();
    vi.mocked(validateNodeGraph).mockResolvedValue({ valid: true, diagnostics: [], edges: [] });
    expect((await validatedApply(graph, 'demo', 'r1', () => false, vi.fn())).state).toBe('stale');
    expect(applyNodeGraph).not.toHaveBeenCalled();
  });
  it('uses the captured revision for an accepted graph and propagates a stale revision rejection', async () => {
    vi.mocked(applyNodeGraph).mockClear();
    vi.mocked(validateNodeGraph).mockResolvedValue({ valid: true, diagnostics: [], edges: [] });
    vi.mocked(putGraphPoolEntry).mockResolvedValue({ name: 'demo', revision: 'new-r1', graph });
    vi.mocked(applyNodeGraph).mockRejectedValueOnce(new Error('revision conflict'));
    await expect(validatedApply(graph, 'demo', 'stale-r1', () => true, vi.fn())).rejects.toThrow('revision conflict');
    expect(putGraphPoolEntry).toHaveBeenCalledWith('demo', 'stale-r1', graph);
    expect(applyNodeGraph).toHaveBeenCalledOnce();
    expect(applyNodeGraph).toHaveBeenCalledWith('demo', 'new-r1');
  });
});
