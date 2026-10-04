/**
 * Validate a post-sign-in destination. Only same-origin, absolute paths are
 * accepted; scheme-relative, backslash and control-character forms that
 * browsers may reinterpret as another origin are rejected outright.
 */
export function safeReturnPath(
  value: string | null | undefined,
  origin: string,
): string | undefined {
  if (
    !value ||
    value.length > 2048 ||
    !value.startsWith('/') ||
    value.startsWith('//') ||
    /[\\\u0000-\u001f\u007f]/.test(value)
  )
    return;
  let target: URL;
  try {
    target = new URL(value, origin);
  } catch {
    return;
  }
  if (
    target.origin !== origin ||
    target.username ||
    target.password ||
    target.pathname === '/login' ||
    target.pathname.startsWith('/logout') ||
    target.pathname.startsWith('/auth/')
  )
    return;
  return target.pathname + target.search;
}

export function loginPath(returnTo?: string) {
  return returnTo ? `/login?next=${encodeURIComponent(returnTo)}` : '/login';
}
