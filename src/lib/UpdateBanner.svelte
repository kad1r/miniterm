<script lang="ts">
  import { update, runDownload, runInstall, dismiss } from "../store/update.svelte"
  import { t } from "../i18n/locale.svelte"

  const percent = $derived(
    update.progress?.total
      ? Math.floor((update.progress.downloaded / update.progress.total) * 100)
      : 0,
  )

  async function onDownload() {
    const path = await runDownload()
    if (path && confirm(t("update.installConfirm"))) await runInstall(path)
  }
</script>

{#if update.status === "available" || update.status === "downloading" || update.status === "ready"}
  <div class="update-banner">
    {#if update.status === "downloading"}
      <span>{t("update.downloading", { percent })}</span>
    {:else if update.status === "ready"}
      <span>{t("update.installReady")}</span>
    {:else}
      <span>{t("update.bannerAvailable", { version: update.info?.latest ?? "" })}</span>
      <button onclick={onDownload}>{t("update.download")}</button>
      <button class="ghost" onclick={dismiss}>{t("update.dismiss")}</button>
    {/if}
  </div>
{/if}

<style>
  .update-banner {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 14px;
    background: var(--bg-raised);
    border-bottom: 1px solid var(--border);
    color: var(--text);
    font-size: calc(12.5px * var(--font-scale, 1));
  }
  .update-banner button {
    padding: 4px 10px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--accent, var(--bg-app));
    color: var(--text);
    cursor: pointer;
    font: inherit;
  }
  .update-banner button.ghost {
    background: transparent;
    color: var(--text-dim);
  }
</style>
