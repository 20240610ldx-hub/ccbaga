#!/usr/bin/env python3
# [INPUT]: 实际生成的 DMG、macOS 只读挂载和签名工具、ds-store 背景元数据读取。
# [OUTPUT]: 包内双应用严格签名/Gatekeeper、Applications 链接和背景完整性校验；失败退出。
# [POS]: 公证前的最终载荷验收，防止 Finder 布局设置破坏已签名应用。
# [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
import hashlib
import os
from pathlib import Path
import subprocess
import sys
import tempfile
from ds_store import DSStore

ROOT = Path(__file__).resolve().parent.parent


def run(*args):
    return subprocess.run([str(arg) for arg in args], check=True, capture_output=True)


def main():
    with tempfile.TemporaryDirectory(prefix='cctool-dmg-verify-') as temporary:
        mount = Path(temporary) / 'mount'
        mount.mkdir()
        run('hdiutil', 'attach', '-readonly', '-nobrowse', '-mountpoint', mount, Path(sys.argv[1]).resolve())
        try:
            if os.readlink(mount / 'Applications') != '/Applications':
                raise SystemExit('DMG Applications shortcut is invalid')
            expected = hashlib.sha256((ROOT / 'assets/dmg-background.png').read_bytes()).digest()
            if hashlib.sha256((mount / '.background.png').read_bytes()).digest() != expected:
                raise SystemExit('DMG pixel background does not match the release asset')
            with DSStore.open(str(mount / '.DS_Store'), 'r') as store:
                if store['.']['icvp']['backgroundType'] != 2:
                    raise SystemExit('Finder image background is not configured')
            for name in ['CC清理器.app', 'CC浏览器.app']:
                app = mount / name
                run('codesign', '--verify', '--deep', '--strict', app)
                run('spctl', '--assess', '--type', 'execute', app)
                print(f'DMG payload verified: {name}', flush=True)
        finally:
            run('hdiutil', 'detach', mount)


if __name__ == '__main__':
    main()
