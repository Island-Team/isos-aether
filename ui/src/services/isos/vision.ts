import { request } from './api-client';

const ORIGIN = typeof ISOSUI_API_ORIGIN === 'undefined' ? 'http://127.0.0.1:16000' : ISOSUI_API_ORIGIN;
export interface VisionBinding { monitor_path: string; transfer_path: string; source_path: string; label: string; interval_ms: number | null }
export interface VisionBindings { run_id: string; revision: string; bindings: VisionBinding[] }
export interface ImageFrame { sample: { source_time_ns: number; clock: string; sequence: number; frame_id: string }; width_px: number; height_px: number; encoding: 'jpeg' | 'rgb8'; data: string }
export interface GraphVisionFrame { op: 'frame'; run_id: string; revision: string; monitor_path: string; transfer_path: string; source_path: string; output_sequence: number; frame: ImageFrame }
export interface BinaryVisionFrame extends Omit<GraphVisionFrame, 'frame'> { sample: ImageFrame['sample']; width_px: number; height_px: number; encoding: 'jpeg' }
const PATH = /^[A-Za-z0-9_.-]+(?:\/[A-Za-z0-9_.-]+){3}$/;
export function validVisionBinding(value: unknown): value is VisionBinding {
  if (!value || typeof value !== 'object') return false;
  const item = value as Partial<VisionBinding>;
  return [item.monitor_path, item.transfer_path, item.source_path].every((path) => typeof path === 'string' && PATH.test(path)) && typeof item.label === 'string' && (item.interval_ms === null || typeof item.interval_ms === 'number' && Number.isSafeInteger(item.interval_ms) && item.interval_ms >= 1);
}
export async function getGraphVisionBindings(): Promise<VisionBindings> {
  const value = await request<VisionBindings>('/vision/bindings');
  if (!value || typeof value.run_id !== 'string' || !value.run_id || !/^[0-9a-f]{64}$/.test(value.revision) || !Array.isArray(value.bindings) || value.bindings.some((item) => !validVisionBinding(item))) throw new Error('Invalid Vision bindings.');
  return value;
}
export function graphVisionSocketUrl(): string {
  const url = new URL(`${ORIGIN}/api/isos/v1/vision/ws`);
  url.protocol = url.protocol === 'https:' ? 'wss:' : 'ws:';
  return url.toString();
}
export function decodeGraphVisionFrame(value: unknown, binding: VisionBinding, runId: string, revision: string): Blob {
  if (!value || typeof value !== 'object') throw new Error('frame_invalid');
  const message = value as Partial<GraphVisionFrame>;
  if (message.op !== 'frame' || message.run_id !== runId || message.revision !== revision || message.monitor_path !== binding.monitor_path || message.transfer_path !== binding.transfer_path || message.source_path !== binding.source_path) throw new Error('frame_identity_mismatch');
  if (!Number.isSafeInteger(message.output_sequence) || (message.output_sequence as number) < 0) throw new Error('frame_invalid');
  const frame = message.frame;
  if (!frame || !['jpeg', 'rgb8'].includes(frame.encoding) || !Number.isSafeInteger(frame.width_px) || !Number.isSafeInteger(frame.height_px) || frame.width_px < 1 || frame.height_px < 1 || !Number.isSafeInteger(frame.sample?.sequence) || frame.sample.sequence < 0 || typeof frame.sample.frame_id !== 'string' || !frame.sample.frame_id || frame.sample.frame_id.length > 128 || typeof frame.data !== 'string' || frame.data.length > 20 * 1024 * 1024 || !/^[A-Za-z0-9+/]*={0,2}$/.test(frame.data)) throw new Error('frame_invalid');
  const bytes = Uint8Array.from(atob(frame.data), (char) => char.charCodeAt(0));
  if (frame.encoding === 'jpeg') {
    if (bytes.length < 4 || bytes[0] !== 0xff || bytes[1] !== 0xd8 || bytes[bytes.length - 2] !== 0xff || bytes[bytes.length - 1] !== 0xd9) throw new Error('jpeg_invalid');
    return new Blob([bytes], { type: 'image/jpeg' });
  }
  const width = frame.width_px; const height = frame.height_px;
  if (!Number.isSafeInteger(width * height * 3) || bytes.length !== width * height * 3) throw new Error('rgb8_invalid');
  const rowBytes = (width * 3 + 3) & ~3;
  const output = new Uint8Array(54 + rowBytes * height);
  const header = new DataView(output.buffer);
  output[0] = 0x42; output[1] = 0x4d; header.setUint32(2, output.length, true); header.setUint32(10, 54, true); header.setUint32(14, 40, true); header.setInt32(18, width, true); header.setInt32(22, -height, true); header.setUint16(26, 1, true); header.setUint16(28, 24, true); header.setUint32(34, rowBytes * height, true);
  for (let y = 0; y < height; y++) for (let x = 0; x < width; x++) { const source = (y * width + x) * 3; const target = 54 + y * rowBytes + x * 3; output[target] = bytes[source + 2]; output[target + 1] = bytes[source + 1]; output[target + 2] = bytes[source]; }
  return new Blob([output], { type: 'image/bmp' });
}

export function decodeBinaryVisionFrame(packet: ArrayBuffer, binding: VisionBinding, runId: string, revision: string): { value: BinaryVisionFrame; blob: Blob } {
  const bytes = new Uint8Array(packet);
  if (bytes.length < 8 || bytes.length > 20 * 1024 * 1024 + 4100) throw new Error('frame_invalid');
  const headerLength = new DataView(packet).getUint32(0, false);
  if (headerLength < 1 || headerLength > 4096 || 4 + headerLength + 4 > bytes.length) throw new Error('frame_invalid');
  const value = JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(bytes.subarray(4, 4 + headerLength))) as BinaryVisionFrame;
  if (value.op !== 'frame' || value.run_id !== runId || value.revision !== revision || value.monitor_path !== binding.monitor_path || value.transfer_path !== binding.transfer_path || value.source_path !== binding.source_path) throw new Error('frame_identity_mismatch');
  if (!Number.isSafeInteger(value.output_sequence) || value.output_sequence < 0 || value.encoding !== 'jpeg' || !Number.isSafeInteger(value.width_px) || value.width_px < 1 || !Number.isSafeInteger(value.height_px) || value.height_px < 1 || !Number.isSafeInteger(value.sample?.sequence) || value.sample.sequence < 0 || typeof value.sample.frame_id !== 'string' || !value.sample.frame_id || value.sample.frame_id.length > 128 || !Number.isSafeInteger(value.sample.source_time_ns) || typeof value.sample.clock !== 'string') throw new Error('frame_invalid');
  const jpeg = bytes.subarray(4 + headerLength);
  if (jpeg.length < 4 || jpeg[0] !== 0xff || jpeg[1] !== 0xd8 || jpeg[jpeg.length - 2] !== 0xff || jpeg[jpeg.length - 1] !== 0xd9) throw new Error('jpeg_invalid');
  return { value, blob: new Blob([jpeg], { type: 'image/jpeg' }) };
}
