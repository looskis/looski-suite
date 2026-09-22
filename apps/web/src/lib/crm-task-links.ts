import type { components } from '$lib/api/schema';
export type TaskFields = {
  title: string;
  notes: string;
  assigned_membership_id: string;
  due_on: string;
  status: string;
};
export type TaskLinkFormResult = {
  message: string;
  draft?: TaskFields & {
    key: string;
    expected_version: string;
    task_id: string;
    mode: string;
  };
  endDraft?: {
    link_id: string;
    ended: string;
    expected_version: string;
    key: string;
  };
};
export type TaskLinksData = {
  workspace: components['schemas']['Workspace'];
  record: { id: string; title: string; version: number; archived: boolean };
  links: components['schemas']['TaskLinkPage'];
  choices: components['schemas']['Task'][];
  search: string;
  ended: boolean;
  sales: string;
  fields: TaskFields;
  createKey: string;
  linkKey: string;
  endKeys: Record<string, string>;
};
