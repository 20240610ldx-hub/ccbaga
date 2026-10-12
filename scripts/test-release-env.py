#!/usr/bin/env python3
# [INPUT]: 发布入口的真实 BUILD_ENV 与伪造环境凭据。
# [OUTPUT]: 构建子进程不能继承 Apple/更新凭据的回归断言。
# [POS]: macOS CI 的无真实凭据测试，验证环境隔离后才允许加载实际签名材料。
# [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
import importlib.util
import os
from pathlib import Path
from unittest.mock import patch

credentials = ['APPLE_API_KEY', 'APPLE_API_ISSUER', 'APPLE_API_KEY_PATH',
               'APPLE_CERTIFICATE_PASSWORD', 'TAURI_SIGNING_PRIVATE_KEY',
               'TAURI_SIGNING_PRIVATE_KEY_PATH', 'TAURI_SIGNING_PRIVATE_KEY_PASSWORD']
with patch.dict(os.environ, {key: 'disposable-fixture' for key in credentials}):
    spec = importlib.util.spec_from_file_location('release', Path(__file__).with_name('release-macos.py'))
    release = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(release)
    assert all(key not in release.BUILD_ENV for key in credentials)
    assert release.BUILD_ENV['PATH'] == os.environ['PATH']
print('Apple and updater credentials are excluded from build environments.')
