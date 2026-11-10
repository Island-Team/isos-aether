import { request } from './api-client';

export interface ButtonActivation {
  schema_version: '0.2.0';
  source_node: string;
  output: 'activation';
  data_type: 'button_activation@v0.2';
  run_id: string;
  active_revision: string;
  payload: { event_id: string; kind: 'activate' };
}

export interface ButtonActivationResult {
  event_id: string;
  command_id: string;
  status: 'accepted' | 'running' | 'succeeded' | 'failed' | 'cancelled' | 'outcome_unknown';
  joint_positions_rad?: number[];
}

export interface ButtonResultQuery {
  schema_version: '0.2.0';
  source_node: string;
  run_id: string;
  active_revision: string;
  event_id: string;
}

export function newButtonEventId(): string {
  return `click-${crypto.randomUUID()}`;
}

function decodeButtonResult(value: unknown, eventId: string): ButtonActivationResult {
  if (!value || typeof value !== 'object' || Array.isArray(value)) throw new Error('IsOS returned an invalid button result.');
  const result = value as Record<string, unknown>;
  if (result.event_id !== eventId || typeof result.command_id !== 'string' || !result.command_id ||
      !['accepted', 'running', 'succeeded', 'failed', 'cancelled', 'outcome_unknown'].includes(String(result.status)) ||
      (result.status === 'succeeded' && (!Array.isArray(result.joint_positions_rad) ||
        result.joint_positions_rad.length !== 6 || !result.joint_positions_rad.every((joint) => typeof joint === 'number' && Number.isFinite(joint))))) {
    throw new Error('IsOS returned an invalid or uncorrelated button result.');
  }
  return result as unknown as ButtonActivationResult;
}

export async function sendButtonActivation(input: ButtonActivation): Promise<ButtonActivationResult> {
  return decodeButtonResult(await request<unknown>('/nodes/events', 'POST', input), input.payload.event_id);
}

export async function getButtonActivationResult(query: ButtonResultQuery, signal?: AbortSignal): Promise<ButtonActivationResult> {
  return decodeButtonResult(await request<unknown>('/nodes/events/result', 'POST', query, signal), query.event_id);
}
