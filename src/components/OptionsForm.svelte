<script lang="ts">
  import type { OptionSpec } from "../lib/types";

  let {
    specs,
    values,
    uid,
    onset,
    onreset,
    sameCount = 0,
    onApplyAll,
  }: {
    specs: OptionSpec[];
    values: Record<string, unknown>;
    uid: string;
    onset: (key: string, value: unknown) => void;
    onreset: () => void;
    sameCount?: number;
    onApplyAll?: () => void;
  } = $props();

  const val = (s: OptionSpec) => values[s.key] ?? s.default;
</script>

<div class="fields">
  {#each specs as s (s.key)}
    {@const id = `${uid}-${s.key}`}
    <div class="field">
      {#if s.kind === "toggle"}
        <label class="check">
          <input type="checkbox" checked={!!val(s)} onchange={(e) => onset(s.key, e.currentTarget.checked)} />
          <span>{s.label}</span>
        </label>
      {:else}
        <label class="title" for={id}>{s.label}</label>
        {#if s.kind === "select"}
          <select {id} value={String(val(s))} onchange={(e) => onset(s.key, e.currentTarget.value)}>
            {#each s.choices ?? [] as c (c.value)}
              <option value={c.value}>{c.label}</option>
            {/each}
          </select>
        {:else if s.kind === "number"}
          <input
            {id}
            type="number"
            min={s.min}
            max={s.max}
            value={val(s) ?? ""}
            placeholder={s.default === null ? "Keep" : ""}
            oninput={(e) => onset(s.key, e.currentTarget.value === "" ? null : Number(e.currentTarget.value))}
          />
          {#if s.unit}<span class="muted small">{s.unit}</span>{/if}
        {:else}
          <input {id} type="text" value={val(s) ?? ""} placeholder={s.placeholder ?? ""} oninput={(e) => onset(s.key, e.currentTarget.value)} />
        {/if}
      {/if}
      {#if s.help}<div class="help">{s.help}</div>{/if}
    </div>
  {/each}
</div>
<div class="actions" style="margin-top: 10px">
  <button class="link" onclick={onreset}>Reset to defaults</button>
  {#if sameCount > 0 && onApplyAll}
    <button class="link" onclick={onApplyAll}>
      Use these options for the {sameCount} other {sameCount === 1 ? "file" : "files"} with the same conversion
    </button>
  {/if}
</div>
