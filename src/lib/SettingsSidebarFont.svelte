<script lang="ts">
  import { appVersion, systemFonts } from "../ipc"
  import {
    DEFAULT_SIDEBAR_FONTS, SIDEBAR_FONTS, fontStack, systemChoice, systemFamily,
    type FontKind, type SidebarFonts,
  } from "../store/sidebarFont"
  import { resetSidebarFonts, setSidebarFont, sidebarFonts } from "../store/sidebarFont.svelte"
  import { t } from "../i18n/locale.svelte"

  const SYSTEM_VALUE = "__system__"

  /** The preview mirrors the sidebar footer, so it shows the running build's version. */
  let version = $state("")
  appVersion().then((v) => (version = v)).catch(() => {})

  /** Installed families, fetched the first time a system font is wanted. */
  let installed = $state<string[] | null>(null)
  /** Field whose "system font" input is open, even before a name is typed. */
  let editing = $state<Record<keyof SidebarFonts, boolean>>({ text: false, mono: false })
  let drafts = $state<Record<keyof SidebarFonts, string>>({
    text: systemFamily(sidebarFonts.text) ?? "",
    mono: systemFamily(sidebarFonts.mono) ?? "",
  })

  async function loadInstalled() {
    if (installed !== null) return
    try {
      installed = await systemFonts()
    } catch {
      installed = []
    }
  }

  function onSelect(field: keyof SidebarFonts, value: string) {
    if (value === SYSTEM_VALUE) {
      editing[field] = true
      void loadInstalled()
      if (drafts[field].trim()) setSidebarFont(field, systemChoice(drafts[field]))
      return
    }
    editing[field] = false
    setSidebarFont(field, value)
  }

  function onDraft(field: keyof SidebarFonts, value: string) {
    drafts[field] = value
    if (value.trim()) setSidebarFont(field, systemChoice(value))
  }

  const isSystem = (field: keyof SidebarFonts) =>
    editing[field] || systemFamily(sidebarFonts[field]) !== null

  const rows: { field: keyof SidebarFonts; kind: FontKind; label: "settings.appearance.sidebarText" | "settings.appearance.sidebarMono" }[] = [
    { field: "text", kind: "sans", label: "settings.appearance.sidebarText" },
    { field: "mono", kind: "mono", label: "settings.appearance.sidebarMono" },
  ]

  const isDefault = $derived(
    sidebarFonts.text === DEFAULT_SIDEBAR_FONTS.text && sidebarFonts.mono === DEFAULT_SIDEBAR_FONTS.mono,
  )
</script>

<div class="sidebar-font">
  <div class="head">
    <strong>{t("settings.appearance.sidebarFont")}</strong>
    <button class="ghost" disabled={isDefault} onclick={() => {
      editing = { text: false, mono: false }
      resetSidebarFonts()
    }}>{t("settings.appearance.reset")}</button>
  </div>

  {#each rows as row (row.field)}
    {@const system = isSystem(row.field)}
    <div class="row">
      <span class="label">{t(row.label)}</span>
      <select
        value={system ? SYSTEM_VALUE : sidebarFonts[row.field]}
        onchange={(e) => onSelect(row.field, e.currentTarget.value)}
        style:font-family={fontStack(sidebarFonts[row.field], row.kind)}
      >
        <optgroup label={t("settings.appearance.fontsSans")}>
          {#each SIDEBAR_FONTS.filter((f) => f.kind === "sans") as f (f.id)}
            <option value={f.id} style:font-family={fontStack(f.id, "sans")}>{f.label}</option>
          {/each}
        </optgroup>
        <optgroup label={t("settings.appearance.fontsMono")}>
          {#each SIDEBAR_FONTS.filter((f) => f.kind === "mono") as f (f.id)}
            <option value={f.id} style:font-family={fontStack(f.id, "mono")}>{f.label}</option>
          {/each}
        </optgroup>
        <option value={SYSTEM_VALUE}>{t("settings.appearance.systemFont")}</option>
      </select>
    </div>
    {#if system}
      <div class="row">
        <span class="label"></span>
        <input
          list="installed-fonts"
          placeholder={t("settings.appearance.systemFontPlaceholder")}
          value={drafts[row.field]}
          onfocus={() => void loadInstalled()}
          oninput={(e) => onDraft(row.field, e.currentTarget.value)}
        />
      </div>
    {/if}
  {/each}

  <datalist id="installed-fonts">
    {#each installed ?? [] as family (family)}
      <option value={family}></option>
    {/each}
  </datalist>
  {#if installed !== null && installed.length === 0}
    <p class="note">{t("settings.appearance.systemFontsUnavailable")}</p>
  {/if}

  <div class="preview" style:font-family="var(--sidebar-font)">
    <span class="ws">{t("settings.appearance.previewFolder")}</span>
    <span class="ws">{t("settings.appearance.previewWorkspace")}</span>
    <span class="figs" style:font-family="var(--sidebar-font-mono)">v{version} · 25:00 · 3/4</span>
  </div>
</div>

<style>
  .sidebar-font {
    display: flex;
    flex-direction: column;
    gap: 10px;
    margin-bottom: 8px;
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg-raised);
  }
  .ghost {
    padding: 6px 12px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: none;
    color: var(--text-dim);
    font: inherit;
    font-size: calc(12px * var(--font-scale, 1));
    cursor: pointer;
  }
  .ghost:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .row {
    display: grid;
    grid-template-columns: 9em 1fr;
    gap: 10px;
    align-items: center;
  }
  .label {
    color: var(--text-dim);
    font-size: calc(12px * var(--font-scale, 1));
  }
  select,
  input {
    width: 100%;
    min-width: 0;
    padding: 6px 9px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg);
    color: var(--text);
    font-size: calc(13px * var(--font-scale, 1));
  }
  select:focus,
  input:focus {
    outline: none;
    border-color: var(--accent);
  }
  .note {
    margin: 0;
    color: var(--text-dim);
    font-size: calc(11.5px * var(--font-scale, 1));
  }
  .preview {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg-sidebar);
    color: var(--text);
    font-size: calc(13px * var(--font-scale, 1));
  }
  .ws:first-child {
    font-weight: 600;
  }
  .figs {
    color: var(--text-dim);
    font-size: calc(11px * var(--font-scale, 1));
  }
</style>
