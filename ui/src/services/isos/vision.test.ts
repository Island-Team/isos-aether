import { describe, expect, it } from 'vitest';
import { decodeBinaryVisionFrame, decodeGraphVisionFrame, validVisionBinding, type VisionBinding } from './vision';

const binding: VisionBinding = { monitor_path: 'isos/ui/vision/front', transfer_path: 'eos/interface/egress/front', source_path: 'eos/hardware/camera/front', label: 'Front', interval_ms: 200 };
const frame = { sample: { source_time_ns: 1, clock: 'simulation', sequence: 2, frame_id: 'front-2' }, width_px: 2, height_px: 1, encoding: 'jpeg', data: btoa(String.fromCharCode(0xff, 0xd8, 0xff, 0xd9)) };
const message = { op: 'frame', run_id: 'run-1', revision: 'a'.repeat(64), ...binding, output_sequence: 0, frame };

describe('graph Vision frame admission', () => {
  it('accepts matching JPEG and RGB8 frames', () => {
    expect(validVisionBinding(binding)).toBe(true);
    expect(decodeGraphVisionFrame(message, binding, 'run-1', message.revision).type).toBe('image/jpeg');
    expect(decodeGraphVisionFrame({ ...message, frame: { ...frame, width_px: 9000 } }, binding, 'run-1', message.revision).type).toBe('image/jpeg');
    const rgb = { ...message, frame: { ...frame, encoding: 'rgb8', data: btoa(String.fromCharCode(255, 0, 0, 0, 255, 0)) } };
    expect(decodeGraphVisionFrame(rgb, binding, 'run-1', message.revision).type).toBe('image/bmp');
  });
  it('accepts the 2048 × 2048 RoboCasa RGB8 maximum', () => {
    const rgb = { ...message, frame: { ...frame, width_px: 2048, height_px: 2048,
      encoding: 'rgb8', data: btoa('\0'.repeat(2048 * 2048 * 3)) } };
    expect(decodeGraphVisionFrame(rgb, binding, 'run-1', message.revision).size).toBe(54 + 2048 * 2048 * 3);
  });
  it('accepts a larger general RGB8 frame within the encoded frame budget', () => {
    const rgb = { ...message, frame: { ...frame, width_px: 2560, height_px: 1600,
      encoding: 'rgb8', data: btoa('\0'.repeat(2560 * 1600 * 3)) } };
    expect(decodeGraphVisionFrame(rgb, binding, 'run-1', message.revision).size).toBe(54 + 2560 * 1600 * 3);
  });
  it('rejects a mismatched binding, malformed bytes, and unsafe pixel counts', () => {
    expect(() => decodeGraphVisionFrame({ ...message, monitor_path: 'isos/ui/vision/rear' }, binding, 'run-1', message.revision)).toThrow('frame_identity_mismatch');
    expect(() => decodeGraphVisionFrame({ ...message, frame: { ...frame, data: btoa('no') } }, binding, 'run-1', message.revision)).toThrow('jpeg_invalid');
    expect(() => decodeGraphVisionFrame({ ...message, frame: { ...frame, width_px: Number.MAX_SAFE_INTEGER + 1 } }, binding, 'run-1', message.revision)).toThrow('frame_invalid');
    expect(() => decodeGraphVisionFrame({ ...message, output_sequence: undefined }, binding, 'run-1', message.revision)).toThrow('frame_invalid');
  });
});

describe('binary graph Vision frames', () => {
  const packet = (overrides: Record<string, unknown> = {}): ArrayBuffer => {
    const header = new TextEncoder().encode(JSON.stringify({ op: 'frame', run_id: 'run-1', revision: 'a'.repeat(64),
      monitor_path: binding.monitor_path, transfer_path: binding.transfer_path, source_path: binding.source_path,
      output_sequence: 3, sample: frame.sample, width_px: 1280, height_px: 720, encoding: 'jpeg', ...overrides }));
    const bytes = new Uint8Array(4 + header.length + 4);
    new DataView(bytes.buffer).setUint32(0, header.length, false);
    bytes.set(header, 4); bytes.set([0xff, 0xd8, 0xff, 0xd9], 4 + header.length);
    return bytes.buffer;
  };
  it('accepts a fenced JPEG packet without base64 conversion', () => {
    const decoded = decodeBinaryVisionFrame(packet(), binding, 'run-1', 'a'.repeat(64));
    expect(decoded.value.width_px).toBe(1280);
    expect(decoded.value.height_px).toBe(720);
    expect(decoded.blob.size).toBe(4);
  });
  it('rejects stale bindings and malformed frame lengths', () => {
    expect(() => decodeBinaryVisionFrame(packet({ source_path: 'eos/hardware/camera/other' }), binding, 'run-1', 'a'.repeat(64))).toThrow('frame_identity_mismatch');
    const malformed = packet(); new DataView(malformed).setUint32(0, 4097, false);
    expect(() => decodeBinaryVisionFrame(malformed, binding, 'run-1', 'a'.repeat(64))).toThrow('frame_invalid');
  });
});
