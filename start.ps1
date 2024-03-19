$logFolderPath = Join-Path -Path $PSScriptRoot -ChildPath "log"

if (-not (Test-Path -Path $logFolderPath)) {
	New-Item -Path $logFolderPath -ItemType Directory | Out-Null
}

$logFileName = "log_$(Get-Date -Format 'yyyyMMdd_HHmmss').log"
$logFilePath = Join-Path -Path $logFolderPath -ChildPath $logFileName

$errFileName = "err_$(Get-Date -Format 'yyyyMMdd_HHmmss').log"
$errFilePath = Join-Path -Path $logFolderPath -ChildPath $errFileName

$exePath = ".\target\release\dwmr-win32.exe"

Start-Process -FilePath $exePath -Wait -RedirectStandardOutput $logFilePath -RedirectStandardError $errFilePath -NoNewWindow

