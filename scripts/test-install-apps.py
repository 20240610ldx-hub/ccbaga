#!/usr/bin/env python3
# [INPUT]: macOS 安装模式边界、开发环境过滤与可丢弃命令替身。
# [OUTPUT]: 开发不能装外部 DMG、只接受临时签名、正式 Gatekeeper 不可绕过和凭据不传构建的回归。 回滚失败保留并报告恢复目录。
# [POS]: 本机安装信任边界验收；不操作 /Applications、钥匙串或真实凭据。
# [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
import contextlib
import importlib.util
import io
import os
import plistlib
import shutil
import tempfile
from pathlib import Path
import subprocess
from types import SimpleNamespace
import unittest
from unittest.mock import patch

def load(name):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(name + '.py'))
    module = importlib.util.module_from_spec(spec); spec.loader.exec_module(module)
    return module

installer = load('install-apps'); dev = load('dev-install')

class InstallTests(unittest.TestCase):
    def test_development_cannot_accept_an_external_installer(self):
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            installer.arguments(['--development', '/Volumes/Installer'])
        self.assertTrue(installer.arguments(['--development']).development)
        self.assertFalse(installer.arguments(['/Volumes/Installer']).development)

    def test_only_local_adhoc_skips_gatekeeper_and_release_still_requires_it(self):
        with patch.object(installer, 'run', return_value=SimpleNamespace(stderr='Signature=adhoc\n')) as run:
            installer.verify(Path('/fixture/app'), True)
            self.assertFalse(any(call.args[0]=='spctl' for call in run.call_args_list))
        with patch.object(installer, 'run', return_value=SimpleNamespace(stderr='Authority=Developer ID\n')):
            with self.assertRaises(RuntimeError): installer.verify(Path('/fixture/app'), True)
        def reject_gatekeeper(*args):
            if args[0]=='spctl': raise subprocess.CalledProcessError(1, args)
            return SimpleNamespace(stderr='Signature=adhoc\n')
        with patch.object(installer, 'run', side_effect=reject_gatekeeper):
            with self.assertRaises(subprocess.CalledProcessError): installer.verify(Path('/fixture/app'))

    def test_development_never_inherits_signing_secrets(self):
        with patch.dict(os.environ, {'APPLE_API_PRIVATE_KEY':'fixture', 'TAURI_SIGNING_PRIVATE_KEY':'fixture', 'TAURI_SIGNING_PRIVATE_KEY_PASSWORD':'fixture', 'CC_SIGN_IDENTITY':'fixture'}):
            env=dev.development_env()
            self.assertEqual(env['CC_SIGN_IDENTITY'], '-')
            self.assertFalse(any(key.startswith(('APPLE_', 'TAURI_SIGNING_PRIVATE_KEY')) for key in env))


    def exercise_transaction(self, failure):
        with tempfile.TemporaryDirectory(prefix='ccbaga-mac-install-') as temporary:
            root = Path(temporary).resolve()
            applications, sources = root / 'Applications', root / 'Installer'
            applications.mkdir()
            for name, bundle_id, _ in installer.APPS:
                for base, content in [(applications, 'old application'), (sources, 'new application')]:
                    app = base / name
                    (app / 'Contents').mkdir(parents=True)
                    (app / 'Contents/Info.plist').write_bytes(plistlib.dumps({'CFBundleIdentifier': bundle_id}))
                    (app / 'Contents/fixture').write_text(content)
            class Paths:
                def __call__(self, value):
                    return applications if str(value) == '/Applications' else Path(value)
                def home(self):
                    return root / 'Home'
            original_mkdtemp, original_rename = tempfile.mkdtemp, Path.rename
            def stage_mkdtemp(*args, **kwargs):
                args = list(args)
                if len(args) >= 3:
                    args[2] = applications
                else:
                    kwargs['dir'] = applications
                return original_mkdtemp(*args, **kwargs)
            def command(*args):
                if args[0] == 'ditto':
                    shutil.copytree(args[1], args[2])
                return SimpleNamespace(stdout='', stderr='')
            def fail_commit_and_restore(path, destination):
                if path.parent.name.startswith('.ccbaga-install-') and (path.name == installer.APPS[0][0] and failure != 'none' or path.name.startswith('previous-') and failure == 'rollback'):
                    raise OSError('fixture install and rollback failure')
                return original_rename(path, destination)
            with patch.object(installer, 'Path', Paths()), patch.object(installer, 'arguments', return_value=SimpleNamespace(folder=str(sources), development=False)), patch.object(installer, 'run', side_effect=command), patch.object(installer.tempfile, 'mkdtemp', side_effect=stage_mkdtemp), patch.object(Path, 'rename', fail_commit_and_restore), patch.object(installer, 'refresh_dock'):
                if failure == 'none':
                    installer.main()
                else:
                    with self.assertRaises((OSError, RuntimeError)) as caught:
                        installer.main()
            stages = list(applications.glob('.ccbaga-install-*'))
            if failure == 'rollback':
                self.assertEqual(len(stages), 1)
                for index in range(len(installer.APPS)):
                    self.assertEqual((stages[0] / f'previous-{index}.app/Contents/fixture').read_text(), 'old application')
                self.assertIn(str(stages[0]), str(caught.exception))
            else:
                self.assertEqual(stages, [])
                for name, _, _ in installer.APPS:
                    self.assertEqual((applications / name / 'Contents/fixture').read_text(), 'new application' if failure == 'none' else 'old application')

    def test_failed_install_and_rollback_keep_all_previous_apps(self):
        self.exercise_transaction('rollback')

    def test_successful_install_removes_recovery_stage(self):
        self.exercise_transaction('none')

    def test_failed_install_restores_previous_apps_and_cleans_stage(self):
        self.exercise_transaction('commit')

if __name__ == '__main__':
    unittest.main()
