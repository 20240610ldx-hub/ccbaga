#!/bin/sh
# [INPUT]: 正式打包的 CCBaga-A/B 与 install-apps.py 的事务安装能力。
# [OUTPUT]: 验签并替换双应用，正常退出旧版本，保留用户资料和隔离属性，再启动宿主。
# [POS]: bun run install:apps 的兼容入口；安装职责统一交给 Python 标准库。
# [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
set -eu
root=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
exec python3 "${root}/scripts/install-apps.py" "$@"
