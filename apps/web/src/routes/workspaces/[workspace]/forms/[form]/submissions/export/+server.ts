import { error } from '@sveltejs/kit';
import { workspaceClient } from '$lib/server/session';
import type { Field } from '$lib/form-definition';
import { answerCell, csvRow, textCell } from '$lib/server/form-csv';
import type { RequestHandler } from './$types';
/**
 * Every row comes from the permission-checked submissions read. With definition
 * access there is a column per question across versions, newest labels first;
 * without it, answers stay together in one JSON column.
 */
export const GET: RequestHandler = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const path = { workspace_id: workspace.id, form_id: event.params.form };
  const read = (after?: string) =>
    client.GET(
      '/api/v1/workspaces/{workspace_id}/forms/{form_id}/submissions',
      { params: { path, query: { after, limit: 100 } } },
    );
  const first = await read();
  if (!first.data)
    error(
      first.response.status,
      first.error?.message ?? 'Submissions unavailable',
    );
  const versions = await client.GET(
    '/api/v1/workspaces/{workspace_id}/forms/{form_id}/versions',
    { params: { path } },
  );
  const columns: Field[] = [];
  for (const version of versions.data ?? [])
    for (const field of version.definition.fields)
      if (
        field.kind !== 'statement' &&
        !columns.some((c) => c.key === field.key)
      )
        columns.push(field);
  const encoder = new TextEncoder();
  const header = [
    'submission_id',
    'submitted_at',
    'version',
    ...(versions.data
      ? columns.map((c) => textCell(c.label))
      : ['answers_json']),
  ];
  type Page = NonNullable<typeof first.data>;
  const rows = (page: Page) =>
    page.items
      .map((submission) => {
        const definition = versions.data?.find(
          (v) => v.id === submission.version_id,
        )?.definition;
        return csvRow([
          submission.id,
          textCell(submission.submitted_at),
          String(submission.version_number),
          ...(versions.data
            ? columns.map((column) =>
                answerCell(
                  definition?.fields.find((f) => f.key === column.key) ??
                    column,
                  submission.answers[column.key],
                ),
              )
            : [textCell(JSON.stringify(submission.answers))]),
        ]);
      })
      .join('');
  let page: Page | null = first.data;
  const body = new ReadableStream<Uint8Array>({
    start(controller) {
      controller.enqueue(encoder.encode(`﻿${csvRow(header)}`));
    },
    // A later failure aborts the download instead of ending it as if complete.
    async pull(controller) {
      if (!page) return controller.close();
      controller.enqueue(encoder.encode(rows(page)));
      if (!page.next_cursor) {
        page = null;
        return;
      }
      const next = await read(page.next_cursor);
      if (!next.data)
        return controller.error(new Error('Submission export stopped'));
      page = next.data;
    },
  });
  return new Response(body, {
    headers: {
      'Content-Type': 'text/csv; charset=utf-8',
      'Content-Disposition': `attachment; filename="looski-form-${event.params.form}-submissions.csv"`,
      'Cache-Control': 'no-store',
      'X-Content-Type-Options': 'nosniff',
    },
  });
};
