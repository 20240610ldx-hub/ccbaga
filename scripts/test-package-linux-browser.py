#!/usr/bin/env python3
# [INPUT]: 真实 Linux 打包入口、可丢弃引擎树与复制/提交故障替身。
# [OUTPUT]: 重建移除旧引擎文件、保留权限/链接/其他资源，并在暂存或提交失败时保留旧构建。
# [POS]: 构建资源回归；非 Linux 仅使用 uname 替身选择 Linux 分支，不安装或启动引擎。
# [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
import importlib.util
import os
from pathlib import Path
import platform
import shutil
import subprocess
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parent.parent
spec = importlib.util.spec_from_file_location('package_linux_browser', ROOT / 'scripts/package-linux-browser.py')
package = importlib.util.module_from_spec(spec)
spec.loader.exec_module(package)


class PackageTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix='ccbaga-package-engine-')
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name).resolve()
        scripts = self.root / 'scripts'
        scripts.mkdir()
        for name in ['package-browser.sh', 'package-linux-browser.py']:
            shutil.copyfile(ROOT / 'scripts' / name, scripts / name)
        self.source = self.root / 'browser-runtime/LinuxBrowser'
        self.source.mkdir(parents=True)
        (self.root / 'browser-runtime/version.txt').write_text('fixture-version\n')
        (self.source / 'version.txt').write_text('fixture-version\n')
        (self.source / 'chrome').write_text('original engine')
        (self.source / 'chrome').chmod(0o755)
        (self.source / 'removed-library.so').write_text('old library')
        (self.source / 'linked-library.so').symlink_to('removed-library.so')
        self.output = self.root / 'target with spaces/release/browser-runtime'
        self.output.mkdir(parents=True)
        (self.output / 'LICENSE.fingerprint-chromium').write_text('keep license')
        self.env = os.environ | {'CARGO_TARGET_DIR': str(self.output.parent.parent)}
        if platform.system() != 'Linux':
            tools = self.root / 'tools'
            tools.mkdir()
            (tools / 'uname').write_text("#!/bin/sh\nprintf '%s\\n' Linux\n")
            (tools / 'uname').chmod(0o755)
            self.env['PATH'] = str(tools) + os.pathsep + os.environ['PATH']

    def run_package(self):
        subprocess.run(['sh', str(self.root / 'scripts/package-browser.sh')], env=self.env, check=True, capture_output=True)

    def test_first_package_preserves_complete_engine_and_other_resources(self):
        self.run_package()
        target = self.output / 'LinuxBrowser'
        self.assertEqual(sorted(path.name for path in target.iterdir()), sorted(path.name for path in self.source.iterdir()))
        self.assertEqual((target / 'version.txt').read_text(), 'fixture-version\n')
        self.assertTrue(os.access(target / 'chrome', os.X_OK))
        self.assertEqual(os.readlink(target / 'linked-library.so'), 'removed-library.so')
        self.assertEqual((self.output / 'LICENSE.fingerprint-chromium').read_text(), 'keep license')

    def test_invalid_source_leaves_old_engine_unchanged(self):
        self.run_package()
        (self.source / 'chrome').chmod(0o644)
        with self.assertRaises(subprocess.CalledProcessError):
            self.run_package()
        self.assertEqual((self.output / 'LinuxBrowser/chrome').read_text(), 'original engine')
        (self.source / 'chrome').chmod(0o755)
        (self.source / 'version.txt').write_text('wrong-version\n')
        with self.assertRaises(subprocess.CalledProcessError):
            self.run_package()
        self.assertEqual((self.output / 'LinuxBrowser/version.txt').read_text(), 'fixture-version\n')

    def test_rebuild_replaces_engine_instead_of_merging_removed_files(self):
        self.run_package()
        (self.source / 'removed-library.so').unlink()
        (self.source / 'linked-library.so').unlink()
        (self.source / 'chrome').write_text('replacement engine')
        (self.source / 'new-library.so').write_text('new library')
        (self.source / 'linked-library.so').symlink_to('new-library.so')
        self.run_package()
        target = self.output / 'LinuxBrowser'
        self.assertEqual(sorted(path.name for path in target.iterdir()), sorted(path.name for path in self.source.iterdir()))
        self.assertEqual((target / 'chrome').read_text(), 'replacement engine')
        self.assertTrue(os.access(target / 'chrome', os.X_OK))
        self.assertEqual(os.readlink(target / 'linked-library.so'), 'new-library.so')
        self.assertEqual((self.output / 'LICENSE.fingerprint-chromium').read_text(), 'keep license')

    def test_copy_failure_preserves_old_engine_and_cleans_stage(self):
        self.run_package()
        with patch.object(package.shutil, 'copytree', side_effect=OSError('fixture copy failure')):
            with self.assertRaises(OSError):
                package.replace_engine(self.source, self.output)
        self.assertEqual((self.output / 'LinuxBrowser/chrome').read_text(), 'original engine')
        self.assertFalse(list(self.output.glob('.ccbaga-engine-*')))

    def test_commit_failure_restores_old_engine(self):
        self.run_package()
        (self.source / 'chrome').write_text('replacement engine')
        original = Path.rename
        def fail_commit(path, destination):
            if path.name == 'LinuxBrowser' and path.parent.name.startswith('.ccbaga-engine-'):
                raise OSError('fixture commit failure')
            return original(path, destination)
        with patch.object(Path, 'rename', fail_commit):
            with self.assertRaises(OSError):
                package.replace_engine(self.source, self.output)
        self.assertEqual((self.output / 'LinuxBrowser/chrome').read_text(), 'original engine')
        self.assertFalse(list(self.output.glob('.ccbaga-engine-*')))


if __name__ == '__main__':
    unittest.main()
