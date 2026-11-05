import type { Cookies } from '@sveltejs/kit';
import type { Theme, Density } from '$lib/appearance.svelte';
export function readAppearance(cookies: Cookies): {
  theme: Theme;
  density: Density;
} {
  const theme = cookies.get('looski_theme');
  return {
    theme: theme === 'dark' || theme === 'light' ? theme : 'system',
    density:
      cookies.get('looski_density') === 'comfortable'
        ? 'comfortable'
        : 'compact',
  };
}
