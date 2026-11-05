import { error } from '@sveltejs/kit';
import { workspaceClient } from '$lib/server/session';
import type { RequestHandler } from './$types';
export const GET: RequestHandler = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const path = { workspace_id: workspace.id, visitor_id: event.params.visitor };
  const detail = await client.GET(
    '/api/v1/workspaces/{workspace_id}/visitors/{visitor_id}',
    { params: { path } },
  );
  if (!detail.data) error(detail.response.status, 'Visitor unavailable');
  // Paginated streaming keeps large exports bounded in server memory.
  let after: string | undefined;
  let first = true;
  const encoder = new TextEncoder();
  const stream = new ReadableStream({
    async pull(controller) {
      try {
        if (first) {
          const { events: _, ...record } = detail.data!;
          controller.enqueue(
            encoder.encode(
              JSON.stringify({ type: 'visitor', ...record }) + '\n',
            ),
          );
          first = false;
        }
        const result = await client.GET(
          '/api/v1/workspaces/{workspace_id}/visitors/{visitor_id}/events',
          { params: { path, query: { after } } },
        );
        if (!result.data) throw new Error('Export interrupted');
        for (const e of result.data.items)
          controller.enqueue(
            encoder.encode(JSON.stringify({ type: 'visit', ...e }) + '\n'),
          );
        after = result.data.next_cursor ?? undefined;
        if (!after) controller.close();
      } catch (e) {
        controller.error(e);
      }
    },
  });
  return new Response(stream, {
    headers: {
      'Content-Type': 'application/x-ndjson',
      'Content-Disposition': `attachment; filename="visitor-${event.params.visitor}.ndjson"`,
      'Cache-Control': 'no-store',
    },
  });
};
