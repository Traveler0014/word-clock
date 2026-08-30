# usb-down.ps1 — 停止 auto-attach 并 detach/unbind 字钟项目的两个 USB 设备
# 用法（管理员 PowerShell）:  .\usb-down.ps1 [-KeepBind]
#   默认: 停 auto-attach 进程 → detach → unbind（完全还原，设备归还 Windows）
#   -KeepBind: 仅 detach，保留 shared 状态（下次 usb-up 免 bind）
#Requires -RunAsAdministrator

param([switch]$KeepBind)

$ErrorActionPreference = 'Continue'
$devices = @('303a:1001', '1a86:55d3')

# 1) 停掉所有 auto-attach 后台进程（否则 detach 后会被立刻重新 attach）
Get-CimInstance Win32_Process -Filter "Name = 'usbipd.exe'" |
    Where-Object { $_.CommandLine -match 'auto-attach' } |
    ForEach-Object {
        Write-Host "stop auto-attach (pid $($_.ProcessId))" -ForegroundColor DarkGray
        Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue
    }

foreach ($id in $devices) {
    Write-Host "==> [$id]" -ForegroundColor Cyan
    $listed = usbipd list | Select-String $id
    if (-not $listed) { Write-Host "    设备不在位，跳过" -ForegroundColor DarkGray; continue }

    # 2) detach（已 attach 才需要）
    if ($listed.Line -match 'Attached') {
        usbipd detach --hardware-id $id
        Write-Host "    detached" -ForegroundColor Green
    }

    # 3) unbind（-KeepBind 跳过）
    if (-not $KeepBind) {
        usbipd unbind --hardware-id $id
        Write-Host "    unbound" -ForegroundColor Green
    } else {
        Write-Host "    保留 shared 状态 (-KeepBind)" -ForegroundColor DarkGray
    }
}

Write-Host ""
usbipd list
