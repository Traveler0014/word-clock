# wsl-port-forward.ps1 — 把 Windows localhost:<port> 转发到 WSL 内服务
# 背景：WSL2 NAT 模式下 localhost 转发在部分机器上不生效（无 .wslconfig），
#       WSL 的 IPv4 每次重启会变，因此用 netsh portproxy 显式转发。
# 用法（管理员）:
#   .\wsl-port-forward.ps1 -Port 8787
#   .\wsl-port-forward.ps1 -Port 8787 -Remove
param(
    [int]$Port = 8787,
    [string]$Distro = 'fedora',
    [switch]$Remove
)
#Requires -RunAsAdministrator

$ErrorActionPreference = 'Stop'

function Get-WslIp {
    # 注意：镜像里没装 hostname 包，`hostname -I` 不可用；用 ip route 取默认路由源地址
    $out = wsl.exe -d $Distro -e /usr/sbin/ip -4 route get 8.8.8.8 2>$null
    $line = ($out | Out-String)
    if ($line -match 'src\s+(\d+\.\d+\.\d+\.\d+)') { return $Matches[1] }
    throw "无法解析 WSL IP（ip route 输出: $line）"
}

$existing = netsh interface portproxy show v4tov4 | Select-String "127\.0\.0\.1\s+$Port\b"
if ($existing) {
    netsh interface portproxy delete v4tov4 listenaddress=127.0.0.1 listenport=$Port | Out-Null
    Write-Host "已删除旧规则 (port $Port)" -ForegroundColor DarkGray
}

if ($Remove) { Write-Host "已清理。"; exit 0 }

$wslIp = Get-WslIp
netsh interface portproxy add v4tov4 listenaddress=127.0.0.1 listenport=$Port connectaddress=$wslIp connectport=$Port | Out-Null
if ($LASTEXITCODE -ne 0) { throw "netsh portproxy add 失败 (rc=$LASTEXITCODE)" }

Write-Host "转发就绪: http://localhost:$Port/  ->  ${wslIp}:$Port" -ForegroundColor Green
Write-Host "（WSL 重启后 IP 会变，重跑本脚本即可）" -ForegroundColor Yellow
netsh interface portproxy show v4tov4 | Select-String "127\.0\.0\.1\s+$Port\b"
