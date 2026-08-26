# Shared directory shortcuts — loaded by both Windows PowerShell 5.1 and PowerShell 7.
# Add new shortcuts HERE only; both shells pick them up automatically.
#
# Each function is just a named jump to a folder. Type the function name in the
# terminal (e.g. `my-project`) to Set-Location there.

function my-project { Set-Location "D:\Development\MyProject\" }
function downloads  { Set-Location "$HOME\Downloads\" }
function repos      { Set-Location "D:\Development\" }
