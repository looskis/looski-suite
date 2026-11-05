declare global {
  namespace App {
    interface Locals {
      workspaceSession?: ReturnType<
        typeof import('$lib/server/session').loadWorkspaceList
      >;
    }
  }
}
export {};
