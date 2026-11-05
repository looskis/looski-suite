<script lang="ts">
  import {
    useAppearance,
    type Theme,
    type Density,
  } from '$lib/appearance.svelte';
  const appearance = useAppearance();
</script>

<fieldset class="preference-group">
  <legend>Appearance</legend>
  <div class="segmented">
    {#each ['light', 'dark', 'system'] as theme}
      <button
        type="button"
        aria-pressed={appearance.theme === theme}
        onclick={() => appearance.setTheme(theme as Theme)}
        >{theme[0].toUpperCase() + theme.slice(1)}</button
      >
    {/each}
  </div>
</fieldset>
<fieldset class="preference-group">
  <legend>Display density</legend>
  <div class="segmented">
    {#each ['compact', 'comfortable'] as density}
      <button
        type="button"
        aria-pressed={appearance.density === density}
        onclick={() => appearance.setDensity(density as Density)}
        >{density[0].toUpperCase() + density.slice(1)}</button
      >
    {/each}
  </div>
</fieldset>

<style>
  .preference-group {
    border: 0;
    padding: 0;
    margin: 16px 0;
  }
  legend {
    color: var(--text-secondary);
    font-size: 0.75rem;
    margin-bottom: 8px;
    padding: 0;
  }
  .segmented {
    display: flex;
    gap: 3px;
    padding: 3px;
    background: var(--surface-canvas);
    border: 1px solid var(--border-default);
    border-radius: 7px;
  }
  button {
    flex: 1;
    padding: 6px 8px;
    min-height: 32px;
    background: transparent;
    color: var(--text-secondary);
    font-size: 0.75rem;
    font-weight: 500;
    border-radius: 4px;
    box-shadow: none;
  }
  button[aria-pressed='true'] {
    background: var(--surface-panel);
    color: var(--text-primary);
    box-shadow: 0 1px 3px #0002;
  }
</style>
