#!/usr/bin/env python3
# [INPUT]: Disposable build trees and user prefixes; Linux installation helper.
# [OUTPUT]: Regression checks for installation isolation, rollback and launcher preservation.
# [POS]: Offline tests, never installing into the actual user's directories.
# [PROTOCOL]: Update this header and CLAUDE.md when changing the contract.
import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('install_linux', Path(__file__).with_name('install-linux.py'))
installer = importlib.util.module_from_spec(spec)
spec.loader.exec_module(installer)


class InstallTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix='ccbaga-test-')
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.build = self.root / 'build'
        self.prefix = self.root / 'prefix with spaces'
        for name in ['requirement-french', 'browser-runtime/LinuxBrowser/chrome', 'browser-ui/browser.html']:
            path = self.build / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text('fixture')
            path.chmod(0o755)

    def test_install_upgrade_preserves_unrelated_user_data(self):
        data = self.prefix / 'share/ai.local.requirement-french/browser/profile/keep'
        data.parent.mkdir(parents=True)
        data.write_text('personal')
        installer.install(self.build, self.prefix)
        installer.install(self.build, self.prefix)
        self.assertEqual(data.read_text(), 'personal')
        self.assertEqual((self.prefix / 'bin/ccbaga-a').read_text(), 'fixture')
        self.assertIn('--browser %U', (self.prefix / 'share/applications/ccbaga-browser.desktop').read_text())
        self.assertTrue((self.prefix / 'share/ccbaga-install/lib/CCBaga-A/browser-runtime/LinuxBrowser/chrome').exists())

    def test_custom_launcher_and_linked_desktop_are_preserved(self):
        launcher = self.prefix / 'bin/ccbaga-a'
        launcher.parent.mkdir(parents=True)
        launcher.write_text('custom')
        with self.assertRaises(RuntimeError):
            installer.install(self.build, self.prefix)
        self.assertEqual(launcher.read_text(), 'custom')
        launcher.unlink()
        desktop = self.prefix / 'share/applications/ccbaga-a.desktop'
        desktop.parent.mkdir(parents=True)
        desktop.symlink_to(launcher)
        with self.assertRaises(RuntimeError):
            installer.install(self.build, self.prefix)
        self.assertTrue(desktop.is_symlink())

    def test_failed_desktop_write_rolls_back_application(self):
        installer.install(self.build, self.prefix)
        (self.build / 'requirement-french').write_text('new')
        original = Path.write_text
        def fail_once(path, *args, **kwargs):
            if path.name == 'ccbaga-browser.desktop':
                raise OSError('fixture failure')
            return original(path, *args, **kwargs)
        with patch.object(Path, 'write_text', fail_once):
            with self.assertRaises(OSError):
                installer.install(self.build, self.prefix)
        self.assertEqual((self.prefix / 'bin/ccbaga-a').read_text(), 'fixture')

    def test_exec_escaping_rejects_newlines_and_escapes_field_codes(self):
        self.assertIn('%%', installer.desktop_exec(Path('/tmp/100%/app')))
        self.assertIn('\\\\$', installer.desktop_exec(Path('/tmp/$secret/app')))
        with self.assertRaises(ValueError):
            installer.desktop_exec(Path('/tmp/app\nmalformed'))


if __name__ == '__main__':
    unittest.main()
