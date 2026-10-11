import { error, fail, redirect } from '@sveltejs/kit';
import { randomUUID } from 'node:crypto';
import { workspaceClient, requireSameOrigin } from '$lib/server/session';
import type { Actions, PageServerLoad } from './$types';
export const load: PageServerLoad = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const path = { workspace_id: workspace.id };
  const archived = event.url.searchParams.get('archived') === 'true';
  const [media, files] = await Promise.all([
    client.GET('/api/v1/workspaces/{workspace_id}/cms/media', {
      params: {
        path,
        query: {
          archived,
          after: event.url.searchParams.get('after') ?? undefined,
          limit: 50,
        },
      },
    }),
    // Members without Files access can still read the library.
    client.GET('/api/v1/workspaces/{workspace_id}/files', {
      params: { path, query: { limit: 100 } },
    }),
  ]);
  if (!media.data)
    error(media.response.status, media.error?.message ?? 'Media unavailable');
  return {
    workspace,
    page: media.data,
    files: files.data?.items ?? [],
    archived,
    key: randomUUID(),
    fileKey: randomUUID(),
  };
};
const allowed = [
  'image/png',
  'image/jpeg',
  'image/gif',
  'image/webp',
  'application/pdf',
];
type Client = Awaited<ReturnType<typeof workspaceClient>>['client'];
async function register(
  client: Client,
  workspace: string,
  key: string,
  body: { file_id: string; title: string; alt_text: string },
) {
  return client.POST('/api/v1/workspaces/{workspace_id}/cms/media', {
    params: {
      path: { workspace_id: workspace },
      header: { 'Idempotency-Key': key },
    },
    body,
  });
}
export const actions: Actions = {
  // The upload stays an ordinary private file; only registration adopts it. A failed
  // upload retries under the same keys; a refused file gets a fresh key so that a
  // different file can be chosen.
  upload: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const form = await event.request.formData();
    const title = String(form.get('title') ?? '');
    const alt_text = String(form.get('alt_text') ?? '');
    const key = String(form.get('idempotency_key') ?? '');
    const fileKey = String(form.get('file_key') ?? '');
    const keep = { input: { title, alt_text }, key, fileKey };
    const file = form.get('file');
    if (!file || typeof file === 'string' || !file.name)
      return fail(400, { message: 'Choose a file to upload', ...keep });
    if (file.size > 8 * 1024 * 1024)
      return fail(400, { message: 'Files must be 8 MiB or smaller', ...keep });
    if (!allowed.includes(file.type))
      return fail(400, {
        message: 'Media must be a PNG, JPEG, GIF, WebP or PDF file',
        ...keep,
      });
    const uploaded = await client.POST(
      '/api/v1/workspaces/{workspace_id}/files',
      {
        params: {
          path: { workspace_id: workspace.id },
          header: { 'Idempotency-Key': fileKey },
        },
        body: {
          filename: file.name,
          media_type: file.type || 'application/octet-stream',
          notes: '',
          owner_membership_id: null,
          content_base64: Buffer.from(await file.arrayBuffer()).toString(
            'base64',
          ),
        },
      },
    );
    if (!uploaded.data)
      return fail(uploaded.response.status, {
        message: uploaded.error?.message ?? 'Could not upload this file',
        ...keep,
      });
    const result = await register(client, workspace.id, key, {
      file_id: uploaded.data.id,
      title: title || file.name,
      alt_text,
    });
    if (!result.data)
      return fail(result.response.status, {
        message: `${result.error?.message ?? 'Could not add this file as media'} The upload remains private in Files.`,
        ...keep,
        fileKey: randomUUID(),
      });
    redirect(303, `/workspaces/${workspace.id}/cms/media/${result.data.id}`);
  },
  register: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const form = await event.request.formData();
    const title = String(form.get('title') ?? '');
    const alt_text = String(form.get('alt_text') ?? '');
    const key = String(form.get('idempotency_key') ?? '');
    const result = await register(client, workspace.id, key, {
      file_id: String(form.get('file_id') ?? ''),
      title,
      alt_text,
    });
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not add this file as media',
        input: { title, alt_text },
        key,
      });
    redirect(303, `/workspaces/${workspace.id}/cms/media/${result.data.id}`);
  },
};
