export type Command = {
  id: string;
  label: string;
  group: 'Actions' | 'Navigation' | 'Appearance';
  keywords: string;
  href?: string;
  theme?: 'light' | 'dark' | 'system';
};
export const createCommands = [
  ['contacts', 'New contact', 'Add person lead'],
  ['organizations', 'New organization', 'Add company business account'],
  ['crm/opportunities', 'New opportunity', 'Add sales pipeline'],
  ['crm/deals', 'New deal', 'Add contract transaction'],
  ['tasks', 'New task', 'Add todo reminder'],
  ['projects', 'New project', 'Add board work'],
  ['forms', 'New form', 'Add form survey'],
  ['files', 'Upload file', 'Add document attachment'],
] as const;
/** Commands navigate to existing forms; opening a command never submits a mutation. */
export function actionCommands(root: string): Command[] {
  return createCommands.map(([path, label, keywords]) => ({
    id: `create:${path}`,
    label,
    keywords,
    group: 'Actions',
    href: `${root}/${path}?action=create`,
  }));
}
export function matchCommands(commands: Command[], query: string): Command[] {
  const tokens = query.toLowerCase().trim().split(/\s+/).filter(Boolean);
  return commands.filter((command) =>
    tokens.every((token) =>
      `${command.label} ${command.keywords} ${command.group}`
        .toLowerCase()
        .includes(token),
    ),
  );
}
