<script lang="ts">
  import changelogSource from "../../CHANGELOG.md?raw"
  import { appVersion } from "../ipc"
  import { groupBlocks, parseChangelog, releaseFor, type Span } from "../store/changelog"
  import { update, runCheck, runDownload, runInstall } from "../store/update.svelte"
  import { t } from "../i18n/locale.svelte"

  /** Settings → About: the running version, its release notes from the
   *  bundled CHANGELOG, the project page and the manual update check. */

  const REPO = "https://github.com/kad1r/miniterm"

  async function checkForUpdates() {
    await runCheck(false)
  }
  async function downloadAndInstall() {
    const path = await runDownload()
    if (path && confirm(t("update.installConfirm"))) await runInstall(path)
  }

  // Parsed once at module-eval cost, not per tab switch: the file is a build
  // constant and cannot change while the app runs.
  const releases = parseChangelog(changelogSource)

  // Empty until the bundle answers. Nothing is rendered from it in the meantime,
  // so there is no flash of a wrong version.
  let version = $state("")
  appVersion().then((v) => (version = v)).catch(() => {})

  const release = $derived(version === "" ? null : releaseFor(releases, version))
</script>

<div class="card about">
  <span class="card-main">
    <strong class="product">miniterm</strong>
    <span class="tagline">{t("settings.about.tagline")}</span>
  </span>
  <span class="version">
    <span class="label">{t("settings.about.version")}</span>
    <output>{version === "" ? t("settings.about.unknownVersion") : version}</output>
    {#if release?.date}
      <small>{t("settings.about.released", { date: release.date })}</small>
    {/if}
  </span>
</div>

<h3>{t("settings.about.notes")}</h3>
{#if release}
  <div class="notes">
    {#each groupBlocks(release.blocks) as group, i (i)}
      {#if group.kind === "heading"}
        <h4>{@render runs(group.block.spans)}</h4>
      {:else if group.kind === "para"}
        <p>{@render runs(group.block.spans)}</p>
      {:else if group.kind === "list"}
        <ul>
          {#each group.items as item, j (j)}
            <li>{@render runs(item.spans)}</li>
          {/each}
        </ul>
      {/if}
    {/each}
  </div>
{:else}
  <p class="hint">{t("settings.about.noNotes", { version })}</p>
{/if}

<h3>{t("settings.about.source")}</h3>
<p class="repo"><code>{REPO}</code></p>

<h3>{t("update.checkButton")}</h3>
<div class="update-check">
  <button
    onclick={checkForUpdates}
    disabled={update.status === "checking" || update.status === "downloading"}
  >
    {update.status === "checking" ? t("update.checking") : t("update.checkButton")}
  </button>
  {#if update.status === "upToDate"}
    <span class="hint">{t("update.upToDate")}</span>
  {:else if update.status === "error"}
    <span class="hint err">{t("update.failed")}</span>
  {:else if update.status === "available"}
    <span class="hint">{t("update.bannerAvailable", { version: update.info?.latest ?? "" })}</span>
    <button onclick={downloadAndInstall}>{t("update.download")}</button>
  {:else if update.status === "downloading"}
    <span class="hint">{t("update.downloading", { percent: update.progress?.total ? Math.floor((update.progress.downloaded / update.progress.total) * 100) : 0 })}</span>
  {:else if update.status === "ready"}
    <span class="hint">{t("update.installReady")}</span>
  {/if}
</div>

<!-- One line on purpose: a newline between the branches would be rendered as a
     space inside the sentence, so "pick **Apply** now" would gain gaps. -->
{#snippet runs(spans: Span[])}{#each spans as s, i (i)}{#if s.style === "bold"}<strong>{s.text}</strong>{:else if s.style === "code"}<code>{s.text}</code>{:else}{s.text}{/if}{/each}{/snippet}

<style>
  /* .card, .card-main, .hint and h3 mirror the Settings tabs' own rules;
     component styles are scoped, so the About tab carries its copy. */
  .hint {
    margin: 0 0 14px;
    color: var(--text-dim);
    font-size: calc(12px * var(--font-scale, 1));
  }
  h3 {
    margin: 22px 0 8px;
    color: var(--text-dim);
    font-size: calc(11px * var(--font-scale, 1));
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  .card {
    display: flex;
    gap: 8px;
    align-items: center;
    margin-bottom: 8px;
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg-raised);
  }
  .card-main {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
  }
  .update-check {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
  }
  .update-check .hint {
    margin: 0;
  }
  .update-check .hint.err {
    color: var(--err);
  }
  .about {
    align-items: flex-start;
    padding: 14px;
  }
  .product {
    font-size: calc(16px * var(--font-scale, 1));
  }
  .tagline {
    color: var(--text-dim);
    font-size: calc(12px * var(--font-scale, 1));
    white-space: normal;
  }
  .version {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 2px;
    text-align: right;
  }
  .version .label {
    color: var(--text-dim);
    font-size: calc(11px * var(--font-scale, 1));
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  .version output {
    font-size: calc(17px * var(--font-scale, 1));
    font-variant-numeric: tabular-nums;
  }
  .version small {
    color: var(--text-dim);
    font-size: calc(11px * var(--font-scale, 1));
  }
  .notes {
    font-size: calc(12px * var(--font-scale, 1));
    line-height: 1.65;
  }
  .notes h4 {
    margin: 16px 0 6px;
    font-size: calc(13px * var(--font-scale, 1));
  }
  .notes h4:first-child {
    margin-top: 0;
  }
  .notes p {
    margin: 0 0 8px;
    color: var(--text-dim);
  }
  .notes ul {
    margin: 0 0 8px;
    padding-left: 18px;
    color: var(--text-dim);
  }
  .notes strong {
    color: var(--text);
  }
  .notes code,
  .repo code {
    padding: 1px 4px;
    border-radius: 4px;
    background: var(--bg-raised);
    font-size: calc(11px * var(--font-scale, 1));
  }
  .repo {
    margin: 0;
    color: var(--text-dim);
    user-select: text;
  }
</style>
