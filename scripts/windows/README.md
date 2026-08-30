# USB 设备管理脚本（Windows PowerShell）

字钟项目双 USB 设备（ESP32-C5 USB-JTAG `303a:1001` + UART CH9102F `1a86:55d3`）的 usbipd 快捷操作。

| 脚本 | 作用 |
|---|---|
| `usb-up.ps1` | bind（如未绑）+ 后台 auto-attach 常驻：断线/重枚举自动重连 |
| `usb-down.ps1` | 停 auto-attach → detach → unbind（还原为 Windows 设备） |
| `usb-down.ps1 -KeepBind` | 仅 detach，保留 shared 状态 |

```powershell
# 管理员 PowerShell，进入本目录
.\usb-up.ps1        # 调试开始时（设备需已插好）
.\usb-down.ps1      # 调试结束 / 归还设备给 Windows 时
```

- auto-attach 以隐藏进程常驻（脚本退出不影响），日志：`%TEMP%\usbipd-auto-attach-*.log`
- 开机自启：把 `usb-up.ps1` 注册为登录时计划任务即可（任务计划程序 → 登录触发 → 操作: `powershell -File <路径>\usb-up.ps1`，需最高权限）
- WSL 侧验证：`lsusb`、`ls /dev/ttyACM*`
