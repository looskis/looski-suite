import { error } from '@sveltejs/kit';
import { workspaceClient } from '$lib/server/session';
import type { RequestHandler } from './$types';
/** Staff view of media through the session; sandboxed like the public route. */
export const GET: RequestHandler = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const result = await client.GET(
    '/api/v1/workspaces/{workspace_id}/cms/media/{media_id}/content',
    {
      params: {
        path: { workspace_id: workspace.id, media_id: event.params.media },
      },
    },
  );
  if (!result.data)
    error(result.response.status, result.error?.message ?? 'Media unavailable');
  const { media, content_base64 } = result.data;
  return new Response(Buffer.from(content_base64, 'base64'), {
    headers: {
      'Content-Type': media.media_type,
      'Content-Disposition': media.media_type.startsWith('image/')
        ? 'inline'
        : 'attachment',
      'X-Content-Type-Options': 'nosniff',
      'Content-Security-Policy': "default-src 'none'; sandbox",
    },
  });
};
