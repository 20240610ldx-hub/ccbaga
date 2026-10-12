#!/usr/bin/env python3
# [INPUT]: Ubuntu x86_64 构建环境、固定引擎、版本与 Tauri Linux 配置。
# [OUTPUT]: 验收后的 DEB/RPM/AppImage 和独立 SHA256 清单。
# [POS]: 无 Apple/更新私钥的 Linux 发布执行层；提取真实产物后逐一核对，不修改用户系统。
# [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
import importlib.util
import json
import os
from pathlib import Path
import platform
import re
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent.parent
spec = importlib.util.spec_from_file_location('payload', ROOT / 'scripts/verify-linux-package.py')
payload = importlib.util.module_from_spec(spec)
spec.loader.exec_module(payload)


def run(*args, **kwargs):
    subprocess.run([str(a) for a in args], check=True, **kwargs)


def main():
    version = os.environ['CC_RELEASE_VERSION']
    if platform.system() != 'Linux' or platform.machine() != 'x86_64' or not re.fullmatch(r'\d+\.\d+\.\d+', version):
        raise SystemExit('Linux x86_64 and a numeric release version are required')
    if any(key.startswith('APPLE_') or key.startswith('TAURI_SIGNING_PRIVATE_KEY') for key in os.environ):
        raise SystemExit('Linux builds must not receive Apple or updater credentials')
    run('bun', 'test')
    run('bun', 'run', 'build')
    run('cargo', 'test', '--locked', '--manifest-path', 'src-tauri/Cargo.toml', '--features', 'custom-protocol')
    run('python3', 'scripts/test-linux.py')
    build_env = os.environ | {'NO_STRIP': 'true', 'APPIMAGE_EXTRACT_AND_RUN': '1'}
    run('bun', 'run', 'tauri', 'build', '--features', 'custom-protocol', '--config', json.dumps({'version': version}), env=build_env)
    output = ROOT / 'releases'
    output.mkdir(exist_ok=True)
    manifests = []
    engine = ROOT / 'browser-runtime/LinuxBrowser/chrome'
    for kind, extension in [('deb', 'deb'), ('rpm', 'rpm'), ('appimage', 'AppImage')]:
        candidates = list((ROOT / f'src-tauri/target/release/bundle/{kind}').glob(f'*.{extension}'))
        if len(candidates) != 1:
            raise RuntimeError(f'Expected one {kind} artifact')
        source = candidates[0]
        with tempfile.TemporaryDirectory(prefix='ccbaga-linux-verify-') as temporary:
            root = Path(temporary)
            if kind == 'deb':
                run('dpkg-deb', '--extract', source, root)
                assert subprocess.check_output(['dpkg-deb', '-f', str(source), 'Version'], text=True).strip() == version
            elif kind == 'rpm':
                run('bsdtar', '-xf', source, '-C', root)
                assert subprocess.check_output(['rpm', '-qp', '--queryformat', '%{VERSION}', str(source)], text=True).strip() == version
            else:
                run(source, '--appimage-extract', cwd=root, stdout=subprocess.DEVNULL)
                root /= 'squashfs-root'
            payload.verify(root, engine)
        target = output / f'CCBaga-{version}-linux-x86_64.{extension}'
        shutil.copy2(source, target)
        manifests.append(f'{payload.digest(target)}  {target.name}\n')
    (output / 'Linux-SHA256SUMS').write_text(''.join(manifests))


if __name__ == '__main__':
    main()
