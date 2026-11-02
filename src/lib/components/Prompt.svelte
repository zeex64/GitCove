<script lang="ts">
  import { app } from '../state/app.svelte'

  let el = $state<HTMLFormElement>()
  let values = $state<Record<string, string>>({})
  let checked = $state(false)
  let pos = $state({ x: 0, y: 0 })

  const p = $derived(app.prompt)
  const error = $derived(p?.validate?.(values) ?? null)
  const empty = $derived(p ? p.fields.some((f) => !(values[f.name] ?? '').trim()) : true)

  $effect(() => {
    if (!p) return
    values = Object.fromEntries(p.fields.map((f) => [f.name, f.value ?? '']))
    checked = p.check?.value ?? false
  })

  $effect(() => {
    if (!p || !el) return
    const r = el.getBoundingClientRect()
    pos = {
      x: Math.max(8, Math.min(p.x, window.innerWidth - r.width - 8)),
      y: Math.min(p.y, window.innerHeight - r.height - 8),
    }
    const input = el.querySelector('input')
    input?.focus()
    input?.select()
  })

  function close() {
    app.prompt = null
  }

  function submit(e: SubmitEvent) {
    e.preventDefault()
    if (!p || error || empty) return
    const v = Object.fromEntries(Object.entries(values).map(([k, x]) => [k, x.trim()]))
    close()
    p.onSubmit(v, checked)
  }
</script>

<svelte:window onmousedown={(e) => p && el && !el.contains(e.target as Node) && close()} />

{#if p}
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <form
    class="prompt"
    bind:this={el}
    style="left:{pos.x}px; top:{pos.y}px"
    onsubmit={submit}
    onkeydown={(e) => e.key === 'Escape' && close()}
  >
    <h3>{p.title}</h3>
    {#each p.fields as f (f.name)}
      <label>
        <span>{f.label}</span>
        <input class="field" class:mono={f.mono} bind:value={values[f.name]} placeholder={f.placeholder} spellcheck="false" autocomplete="off" />
      </label>
    {/each}
    {#if p.check}
      <label class="check">
        <input type="checkbox" bind:checked />
        <span>{p.check.label}</span>
      </label>
    {/if}
    {#if error}<p class="error" role="alert">{error}</p>{/if}
    <div class="actions">
      <button type="button" class="btn quiet" onclick={close}>Cancel</button>
      <button type="submit" class="btn primary" disabled={!!error || empty}>{p.confirm}</button>
    </div>
  </form>
{/if}

<style>
  .prompt {
    position: fixed;
    z-index: 55;
    width: 320px;
    padding: 14px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    background: var(--bg-2);
    border: 1px solid var(--line-strong);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-pop);
    animation: pop 140ms var(--ease-out);
  }

  h3 {
    font-size: 13px;
    font-weight: 600;
  }

  label {
    display: flex;
    flex-direction: column;
    gap: 5px;
  }

  label span {
    font-size: 12px;
    color: var(--text-2);
  }

  .field {
    background: var(--bg-1);
  }

  .check {
    flex-direction: row;
    align-items: center;
    gap: 8px;
  }

  .check input {
    accent-color: var(--accent);
    margin: 0;
  }

  .check span {
    color: var(--text);
    font-size: 13px;
  }

  .error {
    font-size: 12px;
    color: var(--danger);
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 6px;
    margin-top: 2px;
  }

  @keyframes pop {
    from {
      opacity: 0;
      transform: translateY(-4px);
    }
  }
</style>
