#!/usr/bin/env python3
# [INPUT]: 固定 Linux 引擎目录与 CARGO_TARGET_DIR 的 release 资源树。
# [OUTPUT]: 完整替换所属 LinuxBrowser 目录，保留其他资源、符号链接和可执行权限；暂存失败不改变旧引擎，回滚失败保留可恢复备份。
# [POS]: 源码/Arch 构建资源复制边界，不访问用户浏览器资料或安装目录。
# [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
from pathlib import Path
import shutil
import sys
import tempfile


def replace_engine(source, output):
    output.mkdir(parents=True, exist_ok=True)
    target = output / 'LinuxBrowser'
    if target.is_symlink() or (target.exists() and not target.is_dir()):
        raise RuntimeError('Refusing a linked or non-directory browser build output')
    stage = Path(tempfile.mkdtemp(prefix='.ccbaga-engine-', dir=output))
    cleanup = True
    try:
        pending, previous = stage / 'LinuxBrowser', stage / 'previous'
        shutil.copytree(source, pending, symlinks=True)
        if target.exists():
            target.rename(previous)
        try:
            pending.rename(target)
        except BaseException:
            if previous.exists():
                try:
                    previous.rename(target)
                except BaseException as rollback_error:
                    cleanup = False
                    raise RuntimeError(f'Browser build rollback failed; previous engine retained at {previous}') from rollback_error
            raise
    finally:
        if cleanup:
            shutil.rmtree(stage)



if __name__ == '__main__':
    replace_engine(Path(sys.argv[1]), Path(sys.argv[2]))
