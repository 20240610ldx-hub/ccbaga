#!/usr/bin/env python3
# [INPUT]: 固定上游 Fingerprint Chromium macOS DMG/Linux tar.xz、发布者 SHA-256 与平台工具。
# [OUTPUT]: 校验来源与架构后恢复被 Git 排除的 browser-runtime/Browser.app。
# [POS]: 干净检出的引擎准备边界；保留上游完整资源与许可，不下载随时间变化的 latest。
# [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
import hashlib
import platform
import tarfile
import shutil
from pathlib import Path
import plistlib
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent.parent
VERSION = '148.0.7778.215'
URL = f'https://github.com/adryfish/fingerprint-chromium/releases/download/{VERSION}/ungoogled-chromium_{VERSION}-1.1_macos.dmg'
SHA256 = 'b72f091e2e1a7583eed389c4b8e3534ed355e568af8c8bbf8fc30a25e23ca679'


def fetch_linux():
    if platform.machine() not in ('x86_64', 'AMD64'):
        raise SystemExit('The pinned upstream Linux engine only supports x86_64')
    target = ROOT / 'browser-runtime/LinuxBrowser'
    if target.exists():
        raise SystemExit('LinuxBrowser already exists; refusing to overwrite a prepared engine')
    url = f'https://github.com/adryfish/fingerprint-chromium/releases/download/{VERSION}/ungoogled-chromium-{VERSION}-1-x86_64_linux.tar.xz'
    digest = '70d239830332e5820aa34dfcb284161cac0429eee25da642830afe04bda717f4'
    with tempfile.TemporaryDirectory(prefix='ccbaga-engine-') as temporary:
        archive = Path(temporary) / 'browser.tar.xz'
        subprocess.run(['curl', '--fail', '--location', '--retry', '3', '--output', str(archive), url], check=True)
        with archive.open('rb') as stream:
            if hashlib.file_digest(stream, 'sha256').hexdigest() != digest:
                raise SystemExit('Upstream browser SHA-256 mismatch')
        extracted = Path(temporary) / 'extracted'
        with tarfile.open(archive) as tar:
            tar.extractall(extracted, filter='data')
        engines = list(extracted.glob('**/chrome'))
        if len(engines) != 1:
            raise SystemExit('Expected exactly one Linux Chromium executable')
        binary = engines[0]
        with binary.open('rb') as stream:
            elf = stream.read(20)
        if elf[:5] != b'\x7fELF\x02' or elf[18:20] != b'\x3e\x00':
            raise SystemExit('Expected an x86_64 ELF executable')
        actual = subprocess.check_output([str(binary), '--version'], text=True)
        if VERSION not in actual:
            raise SystemExit('Upstream browser version mismatch')
        (binary.parent / 'version.txt').write_text(VERSION + '\n')
        shutil.copytree(binary.parent, target, symlinks=True)
    print(f'Fingerprint Chromium {VERSION}: Linux x86_64 SHA-256 verified')


def main():
    if platform.system() == 'Linux':
        return fetch_linux()
    if platform.system() != 'Darwin':
        raise SystemExit('Only macOS and Linux are supported')
    target = ROOT / 'browser-runtime/Browser.app'
    if target.exists():
        raise SystemExit('Browser.app already exists; refusing to overwrite a prepared engine')
    with tempfile.TemporaryDirectory(prefix='cctool-engine-') as temporary:
        image = Path(temporary) / 'browser.dmg'
        subprocess.run(['curl', '--fail', '--location', '--retry', '3', '--output', str(image), URL], check=True)
        with image.open('rb') as stream:
            if hashlib.file_digest(stream, 'sha256').hexdigest() != SHA256:
                raise SystemExit('Upstream browser SHA-256 mismatch')
        mounted = plistlib.loads(subprocess.check_output(['hdiutil', 'attach', '-readonly', '-nobrowse', '-plist', str(image)]))
        volumes = [Path(item['mount-point']) for item in mounted['system-entities'] if 'mount-point' in item]
        try:
            apps = [app for volume in volumes for app in volume.glob('*.app')]
            if len(apps) != 1:
                raise SystemExit('Expected exactly one browser app in upstream DMG')
            metadata = plistlib.loads((apps[0] / 'Contents/Info.plist').read_bytes())
            if metadata['CFBundleShortVersionString'] != VERSION:
                raise SystemExit('Upstream browser version mismatch')
            binary = apps[0] / 'Contents/MacOS' / metadata['CFBundleExecutable']
            if 'arm64' not in subprocess.check_output(['lipo', '-archs', str(binary)], text=True).split():
                raise SystemExit('Upstream browser does not contain Apple Silicon code')
            subprocess.run(['ditto', str(apps[0]), str(target)], check=True)
        finally:
            for volume in volumes:
                subprocess.run(['hdiutil', 'detach', str(volume)], check=True)
    print(f'Fingerprint Chromium {VERSION}: SHA-256 and arm64 verified')


if __name__ == '__main__':
    main()
