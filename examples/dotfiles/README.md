# Example dotfiles

miniterm spawns **PowerShell 7** (`pwsh.exe`) when available. PowerShell 7 and
Windows PowerShell 5.1 use **separate** profile files, so custom functions
defined in one are not visible in the other. This folder shows a small pattern
that keeps your shortcuts in **one** file loaded by both shells.

## Files

- `PowerShellShortcuts.ps1` — a single source of truth for directory-jump
  functions (and any other custom functions/aliases you want).

## Setup

1. Copy `PowerShellShortcuts.ps1` somewhere stable, e.g. `~/dotfiles/`:

   ```
   ~/dotfiles/PowerShellShortcuts.ps1
   ```

2. Make **both** PowerShell profiles load it. Find their paths with `$PROFILE`
   in each shell; typically:

   - PowerShell 7: `~/Documents/PowerShell/Microsoft.PowerShell_profile.ps1`
   - Windows PowerShell 5.1: `~/Documents/WindowsPowerShell/Microsoft.PowerShell_profile.ps1`

   Add these two lines to **each** profile:

   ```powershell
   $__shortcuts = "$HOME\dotfiles\PowerShellShortcuts.ps1"
   if (Test-Path $__shortcuts) { . $__shortcuts }
   ```

3. Open a **new** miniterm window (profiles load at shell start). Type a
   function name (e.g. `my-project`) to jump to its folder.

## Adding shortcuts later

Edit `PowerShellShortcuts.ps1` only. Both shells — and miniterm — pick up the
change on the next new window. No need to touch the profiles again.

## Optional: version-control it

Turn `~/dotfiles/` into a git repo to back it up and keep history:

```
cd ~/dotfiles
git init
git add -A
git commit -m "add PowerShell shortcuts"
```

Push to a private remote (e.g. GitHub) if you want it available on other
machines via `git clone`.
