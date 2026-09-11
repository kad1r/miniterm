export type Locale = "tr" | "en"

export const LOCALES: readonly Locale[] = ["tr", "en"]

/** What each locale calls itself. Never translated: a language picker is only
 *  useful to someone who cannot read the language currently in effect. */
export const LOCALE_NAMES: Record<Locale, string> = { tr: "Türkçe", en: "English" }

/**
 * English is the source table. Its keys define the contract — the Turkish table
 * below is typed against them, so adding a string without translating it, or
 * leaving a stale key behind, is a compile error rather than a missing label.
 *
 * `{name}`-style placeholders are filled by `translate`.
 */
const EN = {
  "app.loading": "Loading…",
  "app.placeholder": "Select a workspace or create a new one.",
  "app.saveFailed": "Could not save settings: {error}",
  "app.recovered":
    "The settings file could not be read; it was set aside as {backup}. Started with an empty workspace.",
  "app.bootFailed": "Startup error: {error} — changes will not be saved this session.",

  "sidebar.title": "Workspaces",
  "sidebar.expand": "Expand sidebar",
  "sidebar.collapse": "Collapse sidebar",
  "sidebar.newFolder": "New folder",
  "sidebar.newFolderName": "New folder",
  "sidebar.newWorkspace": "New workspace",
  "sidebar.emptyTitle": "No workspaces yet.",
  "sidebar.emptyHint": "Press + to get started.",
  "sidebar.settings": "Settings",
  "sidebar.addTerminal": "Add terminal",
  "sidebar.changeLayout": "Change layout",
  "sidebar.rename": "Rename",
  "sidebar.renamePrompt": "New name",
  "sidebar.delete": "Delete",
  "sidebar.deleted": '"{name}" deleted',
  "sidebar.deleteNoTerminals": '"{name}" will be deleted. No open terminals. Continue?',
  "sidebar.deleteWithTerminals":
    '"{name}" will be deleted. {count} terminals will close. Continue?',
  "sidebar.dropRejected": "Cannot move here",

  "status.running": "running",
  "status.dead": "exited",
  "status.off": "stopped",

  "settings.tab.tools": "AI tools",
  "settings.tab.dirs": "Directories",
  "settings.tab.terminal": "Terminal",
  "settings.tab.appearance": "Appearance",
  "settings.edit": "Edit",
  "settings.delete": "Delete",
  "settings.cancel": "Cancel",
  "settings.save": "Save",
  "settings.browse": "Browse…",
  "settings.tools.hint":
    "miniterm stores no credentials. The AI CLIs handle signing in themselves.",
  "settings.tools.name": "Name",
  "settings.tools.command": "Command",
  "settings.tools.add": "+ Add AI tool",
  "settings.tools.required": "Name and command cannot be empty",
  "settings.tools.deleteConfirm": '"{name}" will be deleted. Continue?',
  "settings.tools.deleteConfirmUsed":
    '"{name}" will be deleted. {count} workspaces will fall back to "Shell only". Continue?',
  "settings.dirs.alias": "Alias",
  "settings.dirs.path": "Path",
  "settings.dirs.add": "+ Add directory shortcut",
  "settings.dirs.required": "Alias and path cannot be empty",
  "settings.dirs.deleteConfirm":
    'The "{alias}" shortcut will be deleted. Workspaces are unaffected. Continue?',
  "settings.dirs.recents": "Recently used",
  "settings.terminal.hint":
    "The default shell is used by workspaces that have not picked one of their own.",
  "settings.terminal.rescan": "Rescan shells",
  "settings.terminal.found": "{count} shells found",
  "settings.appearance.hint":
    "Scales the text in the interface and in open terminals; boxes and margins do not change. Shortcuts: Ctrl +, Ctrl −, Ctrl 0, or Ctrl + mouse wheel.",
  "settings.appearance.fontSize": "Text size",
  "settings.appearance.smaller": "Decrease",
  "settings.appearance.larger": "Increase",
  "settings.appearance.reset": "Reset",
  "settings.appearance.language": "Language",

  "wizard.title": "New workspace",
  "wizard.dir": "Directory",
  "wizard.savedDirs": "Saved directories",
  "wizard.recents": "Recently used",
  "wizard.nameAndShell": "Name and shell",
  "wizard.namePlaceholder": "workspace name",
  "wizard.defaultShell": "Default ({shell})",
  "wizard.aiTool": "AI tool",
  "wizard.shellOnly": "Shell only",
  "wizard.shellOnlySub": "No command is run",
  "wizard.note": "The command runs in <strong>every</strong> terminal of the workspace.",
  "wizard.terminals": "Terminals",
  "wizard.layout": "Layout",
  "wizard.create": "Create",
  "wizard.created": '"{name}" created',

  "layout.title": 'Add a terminal to "{name}"',
  "layout.changeTitle": 'Layout of "{name}"',
  "layout.hint": "Terminals that are already open keep running.",
  "layout.changeHint": "Only the grid changes — every terminal keeps running.",
  "layout.atLimit": "This workspace already has the maximum of {max} terminals.",
  "layout.apply": "Add",
  "layout.applyLayout": "Apply",
  "layout.added": '"{name}": {count} terminal(s) added',
  "layout.changed": '"{name}": layout is now {rows}×{cols}',

  "grid.closePane": "Close terminal {n} (Ctrl+Shift+W)",
  "grid.paneClosed": "Terminal closed",

  "sessions.spawnFailed": "Could not open terminal ({path}): {error}",
  "sessions.noShell": "No usable shell found",

  "preview.noShell": "(no shell selected)",
  "preview.noCommand": "{shell} opens, no command is run",
  "preview.withCommand": '{shell} opens, then "{command}" is typed',

  "term.exitUnknown": "unknown",
  "term.exitNotice": "[process exited: {code}] — press Enter to restart",
} as const

const TR: Record<keyof typeof EN, string> = {
  "app.loading": "Yükleniyor…",
  "app.placeholder": "Bir workspace seç ya da yeni bir tane oluştur.",
  "app.saveFailed": "Ayarlar kaydedilemedi: {error}",
  "app.recovered":
    "Ayar dosyası okunamadı, {backup} olarak kenara alındı. Boş bir çalışma alanıyla başlatıldı.",
  "app.bootFailed": "Başlatma hatası: {error} — bu oturumda değişiklikler kaydedilmeyecek.",

  "sidebar.title": "Workspace'ler",
  "sidebar.expand": "Kenar çubuğunu genişlet",
  "sidebar.collapse": "Kenar çubuğunu daralt",
  "sidebar.newFolder": "Yeni klasör",
  "sidebar.newFolderName": "Yeni klasör",
  "sidebar.newWorkspace": "Yeni workspace",
  "sidebar.emptyTitle": "Henüz workspace yok.",
  "sidebar.emptyHint": "Başlamak için + düğmesine bas.",
  "sidebar.settings": "Ayarlar",
  "sidebar.addTerminal": "Terminal ekle",
  "sidebar.changeLayout": "Düzeni değiştir",
  "sidebar.rename": "Yeniden adlandır",
  "sidebar.renamePrompt": "Yeni ad",
  "sidebar.delete": "Sil",
  "sidebar.deleted": '"{name}" silindi',
  "sidebar.deleteNoTerminals": '"{name}" silinecek. Açık terminal yok. Devam?',
  "sidebar.deleteWithTerminals": '"{name}" silinecek. {count} terminal kapanacak. Devam?',
  "sidebar.dropRejected": "Buraya taşınamaz",

  "status.running": "çalışıyor",
  "status.dead": "kapandı",
  "status.off": "durdu",

  "settings.tab.tools": "AI Araçları",
  "settings.tab.dirs": "Dizinler",
  "settings.tab.terminal": "Terminal",
  "settings.tab.appearance": "Görünüm",
  "settings.edit": "Düzenle",
  "settings.delete": "Sil",
  "settings.cancel": "Vazgeç",
  "settings.save": "Kaydet",
  "settings.browse": "Gözat…",
  "settings.tools.hint":
    "miniterm hiçbir kimlik bilgisi saklamaz. Oturum açma işini AI CLI'ları kendi yapar.",
  "settings.tools.name": "Ad",
  "settings.tools.command": "Komut",
  "settings.tools.add": "+ AI aracı ekle",
  "settings.tools.required": "Ad ve komut boş olamaz",
  "settings.tools.deleteConfirm": '"{name}" silinecek. Devam?',
  "settings.tools.deleteConfirmUsed":
    '"{name}" silinecek. {count} workspace "Sadece shell"e dönecek. Devam?',
  "settings.dirs.alias": "Alias",
  "settings.dirs.path": "Yol",
  "settings.dirs.add": "+ Dizin kısayolu ekle",
  "settings.dirs.required": "Alias ve yol boş olamaz",
  "settings.dirs.deleteConfirm":
    '"{alias}" kısayolu silinecek. Workspace\'ler etkilenmez. Devam?',
  "settings.dirs.recents": "Son kullanılanlar",
  "settings.terminal.hint":
    "Varsayılan shell, kendi shell'ini seçmemiş workspace'ler için kullanılır.",
  "settings.terminal.rescan": "Shell'leri yeniden tara",
  "settings.terminal.found": "{count} shell bulundu",
  "settings.appearance.hint":
    "Arayüzdeki ve açık terminallerdeki yazı boyutunu ölçekler; kutu ve kenar boşlukları değişmez. Kısayol: Ctrl +, Ctrl −, Ctrl 0 ya da Ctrl + fare tekerleği.",
  "settings.appearance.fontSize": "Yazı boyutu",
  "settings.appearance.smaller": "Küçült",
  "settings.appearance.larger": "Büyüt",
  "settings.appearance.reset": "Sıfırla",
  "settings.appearance.language": "Dil",

  "wizard.title": "Yeni workspace",
  "wizard.dir": "Dizin",
  "wizard.savedDirs": "Kayıtlı dizinler",
  "wizard.recents": "Son kullanılanlar",
  "wizard.nameAndShell": "Ad ve shell",
  "wizard.namePlaceholder": "workspace adı",
  "wizard.defaultShell": "Varsayılan ({shell})",
  "wizard.aiTool": "AI aracı",
  "wizard.shellOnly": "Sadece shell",
  "wizard.shellOnlySub": "Hiçbir komut çalıştırılmaz",
  "wizard.note": "Komut, workspace'teki <strong>her</strong> terminalde çalışır.",
  "wizard.terminals": "Terminaller",
  "wizard.layout": "Yerleşim",
  "wizard.create": "Oluştur",
  "wizard.created": '"{name}" oluşturuldu',

  "layout.title": '"{name}" için terminal ekle',
  "layout.changeTitle": '"{name}" düzeni',
  "layout.hint": "Açık olan terminaller çalışmaya devam eder.",
  "layout.changeHint": "Yalnız ızgara değişir, terminallerin hepsi çalışmaya devam eder.",
  "layout.atLimit": "Bu workspace zaten en fazla sayıda ({max}) terminale sahip.",
  "layout.apply": "Ekle",
  "layout.applyLayout": "Uygula",
  "layout.added": '"{name}": {count} terminal eklendi',
  "layout.changed": '"{name}": düzen artık {rows}×{cols}',

  "grid.closePane": "{n}. terminali kapat (Ctrl+Shift+W)",
  "grid.paneClosed": "Terminal kapatıldı",

  "sessions.spawnFailed": "Terminal açılamadı ({path}): {error}",
  "sessions.noShell": "Kullanılabilir shell bulunamadı",

  "preview.noShell": "(shell seçilmedi)",
  "preview.noCommand": "{shell} açılır, komut çalıştırılmaz",
  "preview.withCommand": '{shell} açılır, ardından "{command}" yazılır',

  "term.exitUnknown": "bilinmiyor",
  "term.exitNotice": "[process exited: {code}] — yeniden başlatmak için Enter",
}

export type MessageKey = keyof typeof EN

const MESSAGES: Record<Locale, Record<MessageKey, string>> = { en: EN, tr: TR }

/** Look up a message and fill its `{placeholders}`.
 *  An unknown placeholder is left verbatim rather than blanked, so a typo shows
 *  up in the UI instead of silently swallowing the value. */
export function translate(
  locale: Locale,
  key: MessageKey,
  params?: Record<string, string | number>,
): string {
  const template = MESSAGES[locale][key]
  if (!params) return template
  return template.replace(/\{(\w+)\}/g, (whole, name: string) =>
    name in params ? String(params[name]) : whole,
  )
}

export function isLocale(value: unknown): value is Locale {
  return value === "tr" || value === "en"
}

/** Stored choice first, then the OS/browser preference, then English.
 *  Only the primary subtag is compared, so `tr-TR` and `tr` both match. */
export function resolveLocale(stored: string | null, preferred: readonly string[]): Locale {
  if (isLocale(stored)) return stored
  for (const tag of preferred) {
    const primary = tag.toLowerCase().split("-")[0]
    if (isLocale(primary)) return primary
  }
  return "en"
}

export const LOCALE_KEY = "miniterm.locale"

/** The slice of `Storage` this module needs — narrow so it stays testable in
 *  the node environment, where `localStorage` does not exist. */
export interface LocaleStore {
  getItem(key: string): string | null
  setItem(key: string, value: string): void
}

/** Storage access can throw (private browsing, disabled cookies); a failure
 *  falls back to detection rather than propagating. */
export function readLocale(store: LocaleStore | undefined, preferred: readonly string[]): Locale {
  try {
    return resolveLocale(store?.getItem(LOCALE_KEY) ?? null, preferred)
  } catch {
    return resolveLocale(null, preferred)
  }
}

export function writeLocale(store: LocaleStore | undefined, locale: Locale): void {
  try {
    store?.setItem(LOCALE_KEY, locale)
  } catch {
    // ignored — a preference that fails to persist must not break the switch
  }
}
