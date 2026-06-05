<script lang="ts">
  let {
    value = $bindable(''),
    rows = 8,
    disabled = false,
    spellcheck = false,
    mono = false,
    id,
    class: klass = ''
  }: {
    value?: string;
    rows?: number;
    disabled?: boolean;
    spellcheck?: boolean;
    mono?: boolean;
    id?: string;
    class?: string;
  } = $props();
</script>

<textarea
  {id}
  {rows}
  {disabled}
  {spellcheck}
  bind:value
  class="ark-ta w-full appearance-none bg-[#0a0a0a] p-3 text-xs leading-relaxed text-[#ededed] outline-none disabled:opacity-40 {mono ? 'font-mono' : ''} {klass}"
></textarea>

<style>
  /* custom resize grip via corner; hide default heavy native handle look is OS-controlled,
     so we disable native resize and let height stay fixed/scrollable */
  .ark-ta {
    resize: none;
  }
</style>
