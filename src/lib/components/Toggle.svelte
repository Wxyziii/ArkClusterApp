<script lang="ts">
  let {
    checked = $bindable(false),
    disabled = false,
    label,
    id
  }: { checked?: boolean; disabled?: boolean; label?: string; id?: string } = $props();
</script>

<button
  {id}
  type="button"
  {disabled}
  role="switch"
  aria-checked={checked}
  aria-label={label}
  onclick={() => (checked = !checked)}
  class="relative inline-flex h-6 w-11 shrink-0 items-center rounded-full transition-colors disabled:cursor-not-allowed disabled:opacity-40 {checked ? 'bg-[#3a3a3a]' : 'bg-[#2a2a2a]'}"
>
  <span class="inline-block h-4 w-4 transform rounded-full bg-[#ededed] transition-transform {checked ? 'translate-x-6' : 'translate-x-1'}"></span>
</button>
