#!/usr/bin/env python3
# [INPUT]: macOS 安装模式边界、开发环境过滤与可丢弃命令替身。
# [OUTPUT]: 双应用目录移动前后取消、回滚失败保留备份，以及开发/正式签名和凭据隔离回归。
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


    def exercise_transaction(self, failure, index=0):
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
                    if name == installer.APPS[0][0]:
                        ui = app / 'Contents/Resources/browser-ui'
                        (ui / 'assets').mkdir(parents=True)
                        (ui / 'browser.html').write_text('<script type="module" src="./assets/browser.js"></script>')
                        (ui / 'assets/browser.js').write_text('fixture')
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
                staged = path.parent.name.startswith('.ccbaga-install-')
                if staged and (path.name == installer.APPS[0][0] and failure in ['commit', 'rollback'] or path.name.startswith('previous-') and failure in ['rollback', 'cancel-rollback']):
                    raise OSError('fixture install and rollback failure')
                if staged and path.name == installer.APPS[0][0] and failure == 'cancel-rollback':
                    raise KeyboardInterrupt('fixture cancellation')
                selected = path == applications / installer.APPS[index][0] if failure.startswith('old-') else staged and path.name == installer.APPS[index][0]
                if selected and failure.endswith('before'):
                    raise KeyboardInterrupt('fixture cancellation before rename')
                result = original_rename(path, destination)
                if selected and failure.endswith('after'):
                    raise KeyboardInterrupt('fixture cancellation after successful rename')
                return result
            with patch.object(installer, 'Path', Paths()), patch.object(installer, 'arguments', return_value=SimpleNamespace(folder=str(sources), development=False)), patch.object(installer, 'run', side_effect=command), patch.object(installer.tempfile, 'mkdtemp', side_effect=stage_mkdtemp), patch.object(Path, 'rename', fail_commit_and_restore), patch.object(installer, 'refresh_dock'):
                if failure == 'none':
                    installer.main()
                else:
                    with self.assertRaises((OSError, RuntimeError, KeyboardInterrupt)) as caught:
                        installer.main()
            stages = list(applications.glob('.ccbaga-install-*'))
            if failure in ['rollback', 'cancel-rollback']:
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

    def test_cancelled_moves_restore_both_applications(self):
        for index in range(len(installer.APPS)):
            for phase in ['old-before', 'old-after', 'new-before', 'new-after']:
                with self.subTest(app=index, phase=phase):
                    self.exercise_transaction(phase, index)

    def test_cancelled_commit_and_failed_rollback_keep_both_backups(self):
        self.exercise_transaction('cancel-rollback')

if __name__ == '__main__':
    unittest.main()
