<script lang="ts">
  let {
    value = $bindable(0),
    min,
    max,
    step = 1,
    disabled = false,
    id,
    invalid = false,
    class: klass = ''
  }: {
    value?: number;
    min?: number;
    max?: number;
    step?: number;
    disabled?: boolean;
    id?: string;
    invalid?: boolean;
    class?: string;
  } = $props();

  function clamp(n: number) {
    if (min != null && n < min) return min;
    if (max != null && n > max) return max;
    return n;
  }
  function bump(dir: number) {
    const next = clamp((Number(value) || 0) + dir * step);
    value = next;
  }
</script>

<div class="flex items-stretch overflow-hidden rounded-lg border {invalid ? 'border-[#b5544f]' : 'border-[#2a2a2a]'} {klass}">
  <button
    type="button"
    {disabled}
    onclick={() => bump(-1)}
    aria-label="Decrease"
    class="flex w-8 items-center justify-center bg-[#181818] text-sm text-[#8c8c8c] transition-colors hover:bg-[#222222] hover:text-[#ededed] disabled:opacity-40"
  >−</button>
  <input
    {id}
    type="number"
    {min}
    {max}
    {step}
    {disabled}
    bind:value
    onblur={() => (value = clamp(Number(value) || 0))}
    class="ark-num w-full appearance-none border-x border-[#2a2a2a] bg-[#0a0a0a] px-2.5 py-1.5 text-center font-mono text-sm text-[#ededed] outline-none focus:border-[#3a3a3a] disabled:opacity-40"
  />
  <button
    type="button"
    {disabled}
    onclick={() => bump(1)}
    aria-label="Increase"
    class="flex w-8 items-center justify-center bg-[#181818] text-sm text-[#8c8c8c] transition-colors hover:bg-[#222222] hover:text-[#ededed] disabled:opacity-40"
  >+</button>
</div>

<style>
  /* strip native spinner chrome */
  .ark-num::-webkit-outer-spin-button,
  .ark-num::-webkit-inner-spin-button {
    -webkit-appearance: none;
    margin: 0;
  }
  .ark-num {
    -moz-appearance: textfield;
    appearance: textfield;
  }
</style>
