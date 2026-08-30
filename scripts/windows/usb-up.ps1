# usb-up.ps1 — 绑定并自动 attach 字钟项目的两个 USB 设备到 WSL
# 用法（管理员 PowerShell）:  .\usb-up.ps1
# 行为: 对每个设备 bind(如未绑) + 启动后台 auto-attach（断开/重枚举自动重连）
#       auto-attach 以隐藏进程常驻，脚本退出后继续工作，日志见 %TEMP%\usbipd-*.log
# 停止: usb-down.ps1
#Requires -RunAsAdministrator

$ErrorActionPreference = 'Continue'
$devices = @(
    @{ VidPid = '303a:1001'; Name = 'ESP32-C5 USB-JTAG (烧录/调试)' },
    @{ VidPid = '1a86:55d3'; Name = 'ESP32-C5 UART CH9102F (日志)' }
)

foreach ($dev in $devices) {
    $id  = $dev.VidPid
    $name = $dev.Name
    Write-Host "==> [$id] $name" -ForegroundColor Cyan

    # 设备必须在位（否则只提示跳过）
    $listed = usbipd list | Select-String $id
    if (-not $listed) {
        Write-Warning "    设备不在位（未插入或驱动未就绪），跳过。插入后重新运行本脚本即可。"
        continue
    }

    # bind（已 shared 则跳过）
    if ($listed.Line -match 'Not\s*shared') {
        usbipd bind --hardware-id $id
        if ($LASTEXITCODE -ne 0) { Write-Warning "    bind 失败"; continue }
        Write-Host "    bound" -ForegroundColor Green
    } else {
        Write-Host "    already shared" -ForegroundColor DarkGray
    }

    # 防重复：清掉该设备已有的 auto-attach 进程再启动
    Get-CimInstance Win32_Process -Filter "Name = 'usbipd.exe'" |
        Where-Object { $_.CommandLine -match "auto-attach" -and $_.CommandLine -match $id } |
        ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }

    # 后台 auto-attach（隐藏窗口常驻；日志重定向便于排查）
    $log = Join-Path $env:TEMP "usbipd-auto-attach-$($id -replace ':', '').log"
    Start-Process -WindowStyle Hidden -FilePath usbipd.exe `
        -ArgumentList "attach --wsl --auto-attach --hardware-id $id" `
        -RedirectStandardOutput $log -RedirectStandardError "$log.err"
    Write-Host "    auto-attach started (log: $log)" -ForegroundColor Green
}

Write-Host ""
Write-Host "完成。WSL 侧验证: lsusb / ls /dev/ttyACM*" -ForegroundColor Cyan
usbipd list
