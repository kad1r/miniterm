; Hooked into Tauri's generated installer.nsi through
; bundle.windows.nsis.installerHooks.
;
; Installing a newer version is not a plain install: the installer first chains
; the previous version's uninstaller, and that uninstaller shows its ordinary
; confirm page — including the "Delete application data" checkbox, which wipes
; %APPDATA%\dev.miniterm.app, the directory holding config.json with every saved
; workspace and directory shortcut.
;
; Tauri already suppresses that wipe when its own updater drives the reinstall,
; because the updater passes /UPDATE. Nothing suppresses it when someone simply
; runs a newer installer by hand, so an upgrade can silently take the user's
; settings with it.
;
; NSIS appends `_?=<dir>` to the uninstaller's command line only when the
; uninstaller is chained from an installer; a real uninstall started from Apps &
; features runs the bare UninstallString. That flag is therefore the exact
; "this is an upgrade, not an uninstall" signal, and clearing the checkbox state
; on it leaves the wipe reachable only from a deliberate uninstall.

!macro NSIS_HOOK_PREUNINSTALL
  Push $R0
  Push $R1
  StrCpy $R0 0
  ${Do}
    StrCpy $R1 $CMDLINE 3 $R0
    ${If} $R1 == ""
      ${ExitDo}
    ${EndIf}
    ${If} $R1 == "_?="
      StrCpy $DeleteAppDataCheckboxState 0
      ${ExitDo}
    ${EndIf}
    IntOp $R0 $R0 + 1
  ${Loop}
  Pop $R1
  Pop $R0
!macroend
