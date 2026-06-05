<script lang="ts">
  let {
    value = $bindable(''),
    placeholder,
    disabled = false,
    mono = false,
    id,
    invalid = false,
    autocomplete = 'off',
    spellcheck = false,
    class: klass = '',
    onfocusinput
  }: {
    value?: string;
    placeholder?: string;
    disabled?: boolean;
    mono?: boolean;
    id?: string;
    invalid?: boolean;
    autocomplete?: string;
    spellcheck?: boolean;
    class?: string;
    onfocusinput?: () => void;
  } = $props();
</script>

<input
  {id}
  {placeholder}
  {disabled}
  autocomplete={autocomplete as AutoFill}
  {spellcheck}
  bind:value
  onfocus={onfocusinput}
  class="w-full appearance-none rounded-lg border bg-[#0a0a0a] px-3 py-2 text-sm text-[#ededed] outline-none transition-colors placeholder:text-[#5c5c5c] disabled:cursor-not-allowed disabled:opacity-40 {mono ? 'font-mono' : ''} {invalid ? 'border-[#b5544f] focus:border-[#b5544f]' : 'border-[#2a2a2a] focus:border-[#3a3a3a]'} {klass}"
/>
