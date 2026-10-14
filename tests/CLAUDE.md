# tests/
> L2 | 父级: ../CLAUDE.md
network.html: 网络页独立 Vite 测试入口，不进入生产构建。
network-fixture.tsx: 真实 VpsPanel 与可显式释放的虚构诊断 IPC；成功/部分失败/异常/空返回，无真实联网或导航。
identities.html: 独立 Vite 测试入口，生产 rollup 只含 index/browser，不打包本页。
identities-fixture.tsx: 真实 IdentityPanel/Cromma/Motion/TooltipProvider + Tauri 官方 mockIPC；默认联动与确认关闭的虚构状态、40 个虚构账号，当前槽位更新而非新增，当前卡片去重；仅内存模拟切换/重命名/删除/失败/恢复与中英文，不读取任何本机账号。
执行入口 ../scripts/test-identities-ui.ts 使用 Node.js 原生管道及临时 HOME/XDG/浏览器上下文、阻止外网、保留真实 DOM 输入；不采集截图或用户目录。
[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
