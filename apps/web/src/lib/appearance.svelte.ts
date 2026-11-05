import { getContext, setContext } from 'svelte';
export type Theme = 'light' | 'dark' | 'system';
export type Density = 'compact' | 'comfortable';
export type Appearance = {
  theme: Theme;
  density: Density;
  setTheme: (value: Theme) => void;
  setDensity: (value: Density) => void;
};
const key = Symbol('workspace-appearance');
export const provideAppearance = (value: Appearance) => setContext(key, value);
export const useAppearance = () => getContext<Appearance>(key);
export function savePreference(name: string, value: string) {
  document.cookie = `${name}=${value}; Path=/; Max-Age=31536000; SameSite=Lax${location.protocol === 'https:' ? '; Secure' : ''}`;
}
