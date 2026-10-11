# .github/workflows/
> L2 | 父级: ../CLAUDE.md
release.yml: release push 与手动触发；arm64 runner 哈希安装 DMG 工具、缓存 Rust 依赖、测试并签名公证，临时钥匙串持有 Secrets，固定 SHA Actions；独立最小权限发布任务读取不可变产物，校验实际签名更新包并交付 latest.json，成功后显式固定并核对递增版本 tag，再发布 Release，避免分支推进后隐式建 tag 的权限失败，公开仓库附带来源证明。
[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
