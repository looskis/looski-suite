import { error } from '@sveltejs/kit';
import { createHash } from 'node:crypto';
import { workspaceClient } from '$lib/server/session';
import type { RequestHandler } from './$types';
export const GET: RequestHandler = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const result = await client.GET(
    '/api/v1/workspaces/{workspace_id}/files/{id}/content',
    { params: { path: { workspace_id: workspace.id, id: event.params.file } } },
  );
  if (!result.data)
    error(result.response.status, result.error?.message ?? 'File unavailable');
  const { file, content_base64 } = result.data;
  const bytes = Buffer.from(content_base64, 'base64');
  if (
    bytes.length !== file.byte_length ||
    createHash('sha256').update(bytes).digest('hex') !== file.sha256
  )
    error(503, 'File failed integrity verification');
  const filename = encodeURIComponent(file.filename).replace(
    /[!'()*]/g,
    (c) => '%' + c.charCodeAt(0).toString(16).toUpperCase(),
  );
  return new Response(bytes, {
    headers: {
      'Content-Type': 'application/octet-stream',
      'Content-Disposition': `attachment; filename="download"; filename*=UTF-8''${filename}`,
      'Cache-Control': 'no-store',
      'X-Content-Type-Options': 'nosniff',
      'Content-Length': String(bytes.length),
      'Content-Security-Policy': "sandbox; default-src 'none'",
    },
  });
};
