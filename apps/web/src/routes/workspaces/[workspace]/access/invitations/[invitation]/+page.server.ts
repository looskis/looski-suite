import { env } from '$env/dynamic/private';
import { error, fail } from '@sveltejs/kit';
import { randomUUID } from 'node:crypto';
import { workspaceClient, requireSameOrigin } from '$lib/server/session';
import type { Actions, PageServerLoad } from './$types';
export const load: PageServerLoad = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const result = await client.GET(
    '/api/v1/workspaces/{workspace_id}/access/invitations/{invitation_id}',
    {
      params: {
        path: {
          workspace_id: workspace.id,
          invitation_id: event.params.invitation,
        },
      },
    },
  );
  if (!result.data)
    error(
      result.response.status,
      result.error?.message ?? 'Invitation unavailable',
    );
  const link = result.data.token
    ? `${event.url.origin}/invitations/accept?${new URLSearchParams({ workspace: workspace.id, invitation: event.params.invitation, token: result.data.token })}`
    : null;
  return {
    mailEnabled: env.LOOSKI_AUTH_MAIL_ENABLED === 'true',
    workspace,
    invitation: result.data.invitation,
    link,
    key: randomUUID(),
  };
};
export const actions: Actions = {
  default: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const form = await event.request.formData();
    const draft = { expected_version: Number(form.get('expected_version')) };
    const key = String(form.get('idempotency_key') ?? '');
    if (
      !Number.isSafeInteger(draft.expected_version) ||
      draft.expected_version < 1
    )
      return fail(400, {
        message: 'Reload this invitation before changing it.',
        draft,
        key,
      });
    const intent = String(form.get('intent') ?? '');
    if (!['send', 'revoke'].includes(intent))
      return fail(400, {
        message: 'Choose send or revoke.',
        draft,
        key,
        intent,
      });
    if (intent === 'send') {
      const result = await client.POST(
        '/api/v1/workspaces/{workspace_id}/access/invitations/{invitation_id}/send',
        {
          params: {
            path: {
              workspace_id: workspace.id,
              invitation_id: event.params.invitation,
            },
            header: { 'Idempotency-Key': key },
          },
          body: draft,
        },
      );
      if (!result.data)
        return fail(result.response.status, {
          message: result.error?.message ?? 'Could not queue invitation email',
          draft,
          key,
        });
      return {
        message:
          'Invitation email queued. The recipient will verify their email, set a password if needed, and review workspace access.',
        job: result.data.id,
      };
    }
    const result = await client.POST(
      '/api/v1/workspaces/{workspace_id}/access/invitations/{invitation_id}/revoke',
      {
        params: {
          path: {
            workspace_id: workspace.id,
            invitation_id: event.params.invitation,
          },
          header: { 'Idempotency-Key': key },
        },
        body: draft,
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not revoke invitation',
        draft,
        key,
      });
    return { message: 'Invitation revoked.' };
  },
};
