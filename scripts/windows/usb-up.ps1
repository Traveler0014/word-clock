# usb-up.ps1 — 绑定并自动 attach 字钟项目的两个 USB 设备到 WSL
# 用法（管理员 PowerShell）:  .\usb-up.ps1
# 行为: 对每个设备 bind(如未绑) + 启动后台 auto-attach（断开/重枚举自动重连）
#       auto-attach 以隐藏进程常驻，脚本退出后继续工作，日志见 %TEMP%\usbipd-*.log
# 停止: usb-down.ps1
#
# 失败不静默：任一设备 bind/attach 失败都会在结尾汇总并以 exit 1 退出。
# 已知环境问题（2026-10 实测，Win11 + usbipd-win 5.x）：
#   USBPcap / hrdevmon 等 USB 过滤驱动存在时，裸 `usbipd bind` 会失败，
#   必须 `usbipd bind --force`。脚本先试普通 bind，失败自动回退 --force。
#Requires -RunAsAdministrator

$ErrorActionPreference = 'Continue'
$devices = @(
    @{ VidPid = '303a:1001'; Name = 'ESP32-C5 USB-JTAG (烧录/调试)' },
    @{ VidPid = '1a86:55d3'; Name = 'ESP32-C5 UART CH343/CH9102F (日志)' }
)

$failed = @()
$skipped = @()

foreach ($dev in $devices) {
    $id   = $dev.VidPid
    $name = $dev.Name
    Write-Host "==> [$id] $name" -ForegroundColor Cyan

    # 设备必须在位（否则只提示跳过）
    $listed = usbipd list 2>&1 | Select-String $id
    if (-not $listed) {
        Write-Warning "    设备不在位（未插入 / 线是充电线 / 上游 hub 未枚举），跳过。"
        $skipped += $id
        continue
    }

    # bind（已 shared 则跳过）；失败自动回退 --force
    if ($listed.Line -match 'Not\s*shared') {
        usbipd bind --hardware-id $id 2>&1 | Out-Host
        if ($LASTEXITCODE -ne 0) {
            Write-Warning "    普通 bind 失败 (rc=$LASTEXITCODE)，重试 --force（USB 过滤驱动环境下必需）"
            usbipd bind --force --hardware-id $id 2>&1 | Out-Host
            if ($LASTEXITCODE -ne 0) {
                Write-Warning "    bind --force 仍失败 (rc=$LASTEXITCODE)"
                $failed += "$id bind"
                continue
            }
        }
        Write-Host "    bound" -ForegroundColor Green
    } else {
        Write-Host "    already shared" -ForegroundColor DarkGray
    }

    # 防重复：清掉该设备已有的 auto-attach 进程再启动
    Get-CimInstance Win32_Process -Filter "Name = 'usbipd.exe'" -ErrorAction SilentlyContinue |
        Where-Object { $_.CommandLine -match "auto-attach" -and $_.CommandLine -match $id } |
        ForEach-Object {
            Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue
            Write-Host "    killed stale auto-attach (pid $($_.ProcessId))" -ForegroundColor DarkGray
        }

    # 后台 auto-attach（隐藏窗口常驻；日志重定向便于排查）
    $log = Join-Path $env:TEMP "usbipd-auto-attach-$($id -replace ':', '').log"
    Start-Process -WindowStyle Hidden -FilePath usbipd.exe `
        -ArgumentList "attach --wsl --auto-attach --hardware-id $id" `
        -RedirectStandardOutput $log -RedirectStandardError "$log.err" `
        -ErrorAction SilentlyContinue
    if ($?) {
        Write-Host "    auto-attach started (log: $log)" -ForegroundColor Green
    } else {
        Write-Warning "    auto-attach 启动失败"
        $failed += "$id attach"
    }
}

Write-Host ""
usbipd list

Write-Host ""
if ($failed.Count -gt 0) {
    Write-Host "失败: $($failed -join ', ')" -ForegroundColor Red
    if ($skipped.Count -gt 0) { Write-Host "跳过(不在位): $($skipped -join ', ')" -ForegroundColor Yellow }
    Write-Host "WSL 侧验证: lsusb / ls /dev/ttyACM*" -ForegroundColor Cyan
    exit 1
}

if ($skipped.Count -gt 0) {
    Write-Host "警告: 以下设备不在位，未绑定: $($skipped -join ', ')" -ForegroundColor Yellow
    exit 2
}

Write-Host "完成。WSL 侧验证: lsusb / ls /dev/ttyACM*" -ForegroundColor Green
