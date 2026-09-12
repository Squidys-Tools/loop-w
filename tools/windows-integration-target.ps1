param(
    [Parameter(Mandatory = $true)]
    [string] $Title,
    [int] $Left = 160,
    [int] $Top = 160
)

# Disposable, harness-owned Win32 target. The parent harness is the only
# caller and terminates this process during cleanup; no existing user window
# is selected or modified by the integration checks.
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms

$form = New-Object System.Windows.Forms.Form
$form.Text = $Title
$form.StartPosition = [System.Windows.Forms.FormStartPosition]::Manual
$form.Location = New-Object System.Drawing.Point($Left, $Top)
$form.Size = New-Object System.Drawing.Size(520, 320)
$form.MinimumSize = New-Object System.Drawing.Size(320, 220)
$form.ShowInTaskbar = $true
$form.TopMost = $false
$form.Add_Shown({ $form.Activate() })

[System.Windows.Forms.Application]::Run($form)
