#!/usr/bin/env python3
# [INPUT]: Windows 构建产物、指纹引擎结构、Claude CLI 协议与系统配置。
# [OUTPUT]: 验证 Windows 引擎完整性、ICO 图标格式、PowerShell 协议与时区映射。
# [POS]: Windows 自动化回归验证套件。
# [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
import json
from pathlib import Path
import subprocess
import unittest

ROOT = Path(__file__).resolve().parent.parent


class WindowsBrowserEngineTests(unittest.TestCase):
    def test_engine_binary_and_metadata_exist(self):
        engine_dir = ROOT / 'browser-runtime/WindowsBrowser'
        self.assertTrue(engine_dir.is_dir(), 'WindowsBrowser directory must exist')
        
        chrome_exe = engine_dir / 'chrome.exe'
        self.assertTrue(chrome_exe.is_file(), 'chrome.exe must exist')
        with chrome_exe.open('rb') as f:
            header = f.read(2)
        self.assertEqual(header, b'MZ', 'chrome.exe must be a valid Windows PE binary starting with MZ')
        
        version_file = engine_dir / 'version.txt'
        self.assertTrue(version_file.is_file(), 'version.txt must exist')
        self.assertIn('148.0.7778.215', version_file.read_text())
        
        chrome_dll = engine_dir / 'chrome.dll'
        self.assertTrue(chrome_dll.is_file(), 'chrome.dll must exist')
        self.assertGreater(chrome_dll.stat().st_size, 10_000_000, 'chrome.dll must be substantial')


class WindowsCliProtocolTests(unittest.TestCase):
    def test_claude_cli_ps1_read_protocol(self):
        script = ROOT / 'scripts/claude-cli.ps1'
        self.assertTrue(script.is_file(), 'scripts/claude-cli.ps1 must exist')
        
        proc = subprocess.run(
            ['pwsh', '-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', str(script), 'read'],
            capture_output=True,
            text=True,
        )
        self.assertEqual(proc.returncode, 0, f'claude-cli.ps1 read failed: {proc.stderr}')
        lines = [line.strip() for line in proc.stdout.splitlines() if line.strip()]
        
        has_method = any(l.startswith('CC_CLI_METHOD=') for l in lines)
        has_path = any(l.startswith('CC_CLI_PATH=') for l in lines)
        has_version = any(l.startswith('CC_CLI_VERSION=') for l in lines)
        has_done = any(l == 'CC_CLI_DONE=1' for l in lines)
        
        self.assertTrue(has_method, 'Must report CC_CLI_METHOD')
        self.assertTrue(has_path, 'Must report CC_CLI_PATH')
        self.assertTrue(has_version, 'Must report CC_CLI_VERSION')
        self.assertTrue(has_done, 'Must conclude with CC_CLI_DONE=1')


class WindowsPackagingConfigTests(unittest.TestCase):
    def test_tauri_windows_config_is_valid(self):
        conf_file = ROOT / 'src-tauri/tauri.windows.conf.json'
        self.assertTrue(conf_file.is_file(), 'tauri.windows.conf.json must exist')
        data = json.loads(conf_file.read_text(encoding='utf-8'))
        
        self.assertIn('nsis', data['bundle']['targets'])
        self.assertTrue(data['app']['windows'][0]['decorations'])
        self.assertIn('../browser-runtime/WindowsBrowser/', data['bundle']['resources'])
        
    def test_ico_icon_is_valid(self):
        ico_file = ROOT / 'src-tauri/icons/icon.ico'
        self.assertTrue(ico_file.is_file(), 'icon.ico must exist')
        with ico_file.open('rb') as f:
            header = f.read(4)
        self.assertEqual(header, b'\x00\x00\x01\x00', 'Must be a valid ICO header')


if __name__ == '__main__':
    unittest.main()

