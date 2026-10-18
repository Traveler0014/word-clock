# open-debug.ps1 — 用正确地址在 Windows 浏览器打开字钟调试页
# 为什么需要它：WSL2 的 localhost 转发在本机不生效，且 WSL 的 IPv4 每次重启会变，
#              所以每次现取 IP 再拼 URL。（不需要管理员）
# 用法: powershell -ExecutionPolicy Bypass -File .\open-debug.ps1 [-Port 8787]
param([int]$Port = 8787, [string]$Distro = 'fedora')
$ErrorActionPreference = 'Stop'

# 镜像里没装 hostname 包，用 ip route 取默认路由的源地址
$out  = wsl.exe -d $Distro -e /usr/sbin/ip -4 route get 8.8.8.8 2>$null | Out-String
if ($out -notmatch 'src\s+(\d+\.\d+\.\d+\.\d+)') { throw "无法解析 WSL IP: $out" }
$wslIp = $Matches[1]
$url   = "http://${wslIp}:$Port/"

try {
    $null = Invoke-WebRequest $url -UseBasicParsing -TimeoutSec 4
} catch {
    Write-Warning "服务没响应 ($url)。先在 WSL 里跑: python3 tools/debug_web.py --port $Port"
    exit 1
}

Write-Host "打开 $url" -ForegroundColor Green
Start-Process $url
