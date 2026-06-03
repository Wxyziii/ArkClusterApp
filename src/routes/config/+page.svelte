<script lang="ts">
  import {
    PageHeader, Card, ConfigValueEditor, RawConfigEditorMock, Button, StatusBadge,
    ConfirmActionDialog, SafetyWarningPanel, RestartRequiredBanner
  } from '$lib/components';
  import { configFields, rawGameIni, rawGusIni } from '$lib/data/mock';

  let mode = $state<'form' | 'raw'>('form');
  let fields = $state(structuredClone(configFields));
  let gameIni = $state(rawGameIni);
  let gusIni = $state(rawGusIni);
  let target = $state<'Home' | 'Travel maps' | 'All maps'>('All maps');
  let backupFirst = $state(true);
  let confirmOpen = $state(false);

  let groups = $derived([...new Set(fields.map((f) => f.group))]);
  let dirty = $derived(
    JSON.stringify(fields.map((f) => f.value)) !== JSON.stringify(configFields.map((f) => f.value)) ||
      gameIni !== rawGameIni ||
      gusIni !== rawGusIni
  );
  let restartNeeded = $derived(
    fields.some((f, i) => f.restartRequired && f.value !== configFields[i].value) || gusIni !== rawGusIni || gameIni !== rawGameIni
  );
  let invalidCount = $derived(
    fields.filter((f) => f.type === 'number' && typeof f.value === 'number' && ((f.min != null && f.value < f.min) || (f.max != null && f.value > f.max))).length
  );

  function rollback() {
    fields = structuredClone(configFields);
    gameIni = rawGameIni;
    gusIni = rawGusIni;
  }
</script>

<PageHeader title="Config Editor" icon="⚙️" subtitle="Safe ARK Game.ini / GameUserSettings.ini editing (mock)">
  {#snippet actions()}
    {#if dirty}<StatusBadge label="unsaved changes" tone="amber" dot />{/if}
    <div class="flex rounded-lg border border-[#2a2a2a] p-0.5">
      <button class="rounded px-2.5 py-1 text-xs {mode === 'form' ? 'bg-[#222222] text-[#7c9a82]' : 'text-[#8c8c8c]'}" onclick={() => (mode = 'form')}>Safe Form</button>
      <button class="rounded px-2.5 py-1 text-xs {mode === 'raw' ? 'bg-[#222222] text-[#7c9a82]' : 'text-[#8c8c8c]'}" onclick={() => (mode = 'raw')}>Advanced Raw</button>
    </div>
  {/snippet}
</PageHeader>

{#if restartNeeded}<div class="mb-5"><RestartRequiredBanner reason="Some changed values require a restart." /></div>{/if}

<div class="mb-5">
  <SafetyWarningPanel tone="info" title="Changes are sandboxed">
    Saving always creates a backup first. No raw file paths or shell commands are exposed. Values are validated before apply.
  </SafetyWarningPanel>
</div>

{#if mode === 'form'}
  <div class="space-y-5">
    {#each groups as g (g)}
      <Card title={g} icon="🎚️">
        <div class="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3">
          {#each fields as field, i (field.key)}
            {#if field.group === g}<ConfigValueEditor bind:field={fields[i]} />{/if}
          {/each}
        </div>
      </Card>
    {/each}
  </div>
{:else}
  <div class="grid grid-cols-1 gap-5 lg:grid-cols-2">
    <RawConfigEditorMock filename="Game.ini" bind:value={gameIni} original={rawGameIni} />
    <RawConfigEditorMock filename="GameUserSettings.ini" bind:value={gusIni} original={rawGusIni} />
  </div>
{/if}

<!-- save bar -->
<div class="card-elevated mt-5 flex flex-wrap items-center justify-between gap-3 p-4">
  <div class="flex flex-wrap items-center gap-4">
    <label class="flex items-center gap-2 text-xs text-[#8c8c8c]">
      Apply to
      <select bind:value={target} class="rounded-lg border border-[#2a2a2a] bg-[#0a0a0a] px-2 py-1 text-xs text-[#ededed]">
        <option>Home</option><option>Travel maps</option><option>All maps</option>
      </select>
    </label>
    <label class="flex items-center gap-2 text-xs text-[#8c8c8c]">
      <input type="checkbox" bind:checked={backupFirst} class="accent-[#3a3a3a]" /> Backup before save
    </label>
    {#if invalidCount}<StatusBadge label="{invalidCount} invalid value(s)" tone="red" />{/if}
  </div>
  <div class="flex gap-2">
    <Button variant="ghost" disabled={!dirty} onclick={rollback}>Rollback</Button>
    <Button variant="primary" disabled={!dirty || invalidCount > 0} onclick={() => (confirmOpen = true)}>Save config</Button>
  </div>
</div>

<ConfirmActionDialog bind:open={confirmOpen} title="Save configuration to {target}?" tone="warn" confirmLabel="Save & apply" onconfirm={() => {}}>
  {#snippet body()}
    <p>Applying changed values to <strong>{target}</strong>.</p>
    <ul class="space-y-1 text-xs text-[#8c8c8c]">
      <li>{backupFirst ? '✓ A config backup will be taken first.' : '⚠️ No backup selected — not recommended.'}</li>
      {#if restartNeeded}<li class="text-[#bfa15e]">🔄 Some values require a map restart to take effect.</li>{/if}
    </ul>
  {/snippet}
</ConfirmActionDialog>
