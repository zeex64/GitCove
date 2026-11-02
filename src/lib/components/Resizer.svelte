<script lang="ts">
  let {
    value = $bindable(),
    min,
    max,
    invert = false,
    onend,
  }: { value: number; min: number; max: number; invert?: boolean; onend?: () => void } = $props()

  let dragging = $state(false)

  function down(e: PointerEvent) {
    const start = e.clientX
    const startValue = value
    dragging = true
    const el = e.currentTarget as HTMLElement
    el.setPointerCapture(e.pointerId)
    const move = (ev: PointerEvent) => {
      const dx = (ev.clientX - start) * (invert ? -1 : 1)
      value = Math.round(Math.max(min, Math.min(max, startValue + dx)))
    }
    const up = () => {
      dragging = false
      el.removeEventListener('pointermove', move)
      el.removeEventListener('pointerup', up)
      onend?.()
    }
    el.addEventListener('pointermove', move)
    el.addEventListener('pointerup', up)
  }

  function key(e: KeyboardEvent) {
    const step = e.shiftKey ? 40 : 10
    if (e.key === 'ArrowLeft') value = Math.max(min, value - step * (invert ? -1 : 1))
    else if (e.key === 'ArrowRight') value = Math.min(max, value + step * (invert ? -1 : 1))
    else return
    e.preventDefault()
    onend?.()
  }
</script>

<!-- A focusable separator is the ARIA window-splitter pattern. -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
<div
  class="resizer"
  class:dragging
  role="separator"
  aria-orientation="vertical"
  aria-valuenow={value}
  aria-valuemin={min}
  aria-valuemax={max}
  tabindex="0"
  onpointerdown={down}
  onkeydown={key}
></div>

<style>
  .resizer {
    position: relative;
    width: 1px;
    background: var(--line);
    cursor: col-resize;
    z-index: 2;
  }

  .resizer::after {
    content: '';
    position: absolute;
    inset: 0 -4px;
  }

  .resizer:hover,
  .resizer.dragging,
  .resizer:focus-visible {
    background: var(--accent);
    outline: none;
  }
</style>
