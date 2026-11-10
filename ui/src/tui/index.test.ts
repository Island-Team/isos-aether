import { describe, expect, it } from 'vitest';
import { parseArgs } from './index';

describe('TUI arguments', () => {
  it('uses a local IsOS default and accepts explicit locale', () => {
    expect(parseArgs([])).toEqual({ origin: 'http://127.0.0.1:16000', locale: 'en' });
    expect(parseArgs(['--origin', 'http://127.0.0.1:16080', '--locale', 'cn'])).toEqual({ origin: 'http://127.0.0.1:16080', locale: 'cn' });
  });
  it('rejects unsupported input', () => {
    expect(() => parseArgs(['--origin', 'http://example.com:16000'])).toThrow();
    expect(() => parseArgs(['--locale', 'fr'])).toThrow();
  });
});
