import { request } from './api-client';

export interface ExternalNodeRequest {
  schema_version: 1;
  node_path: string;
  run_id: string;
  active_revision: string;
  op: 'input' | 'read';
  port: string;
  payload: unknown;
}

export interface ExternalNodeReply {
  node_path: string;
  port: string;
  run_id: string;
  active_revision: string;
  payload: unknown;
}

export function callExternalNode(input: ExternalNodeRequest): Promise<ExternalNodeReply> {
  return request('/nodes/external', 'POST', input);
}
