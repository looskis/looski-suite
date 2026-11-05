<script lang="ts">
  import { money, frequency } from '$lib/format';
  let {
    amount,
    currency,
    billingFrequency,
  }: {
    amount?: string | null;
    currency?: string | null;
    billingFrequency?: string;
  } = $props();
</script>

<span class="money-value" title={money(amount, currency ?? 'USD', false)}
  ><span>{money(amount, currency ?? 'USD')}</span
  >{#if amount !== null && amount !== undefined && amount !== ''}<small
      >{frequency(billingFrequency)}</small
    >{/if}</span
>

<style>
  .money-value {
    display: inline-flex;
    flex-direction: column;
    gap: 3px;
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
  }
  .money-value small {
    font-size: 10px;
    color: var(--text-secondary);
  }
</style>
