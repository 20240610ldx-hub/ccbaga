#!/usr/bin/env python3
# [INPUT]: 已构建 NSIS 安装器、固定引擎逐文件哈希与空白 GitHub Windows runner。
# [OUTPUT]: 当前用户安装、实际 A/B 回环启动、升级、卸载和用户资产保留的运行验收。
# [POS]: 只在可丢弃 CI runner 执行安装；拒绝已有应用资料，不读取开发者用户数据。
# [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time
import urllib.request

ROOT=Path(__file__).resolve().parent.parent

def wait_for(check, seconds=45):
    deadline=time.monotonic()+seconds
    while time.monotonic()<deadline:
        if check():return
        time.sleep(.25)
    raise AssertionError('Windows runtime verification timed out')

def main():
    if os.name!='nt' or os.environ.get('GITHUB_ACTIONS')!='true':
        raise SystemExit('Installer mutation is restricted to a disposable GitHub Windows runner')
    installer=Path(sys.argv[1]).resolve()
    manifest=json.loads(Path(sys.argv[2]).read_text(encoding='utf-8'))
    data=Path(os.environ['APPDATA'])/'ai.local.requirement-french'
    if data.exists():raise SystemExit('Refusing to touch pre-existing CCBaga application data')
    sys.path.insert(0,str(ROOT/'skills/ccbaga-network/scripts'))
    import windows
    with tempfile.TemporaryDirectory(prefix='ccbaga-installer-') as temporary:
        folder=Path(temporary)/'应用 with spaces'
        home=Path(temporary)/'fixture-home';home.mkdir()
        skill=home/'.claude/skills/personal/SKILL.md';skill.parent.mkdir(parents=True);skill.write_text('keep-skill',encoding='utf-8')
        settings=home/'.claude/settings.json';settings.write_text('{"keep":true}',encoding='utf-8')
        profile=data/'browser/profile/Default/personal-data';profile.parent.mkdir(parents=True);profile.write_bytes(b'keep-profile')
        env={k:v for k,v in os.environ.items() if not k.startswith(('APPLE_','TAURI_SIGNING_PRIVATE_KEY'))}
        env.update(USERPROFILE=str(home),HOME=str(home),CC_CLI_USER_DIR=str(home),HTTPS_PROXY='http://secret-marker.invalid:1')
        def install():
            subprocess.run(f'"{installer}" /S /D={folder}',check=True,timeout=180,env=env)
        def payload():
            assert (folder/'requirement-french.exe').is_file()
            engine=folder/'browser-runtime/WindowsBrowser'
            if not engine.is_dir():engine=folder/'WindowsBrowser'
            actual={p.relative_to(engine).as_posix():hashlib.file_digest(p.open('rb'),'sha256').hexdigest() for p in engine.rglob('*') if p.is_file()}
            assert actual==manifest,'Installed Chromium payload differs from the verified upstream engine'
            assert any(folder.rglob('browser.html')),'Missing embedded browser page'
            assert any(folder.rglob('browser.ico')),'Missing companion icon'
            assert any(folder.rglob('LICENSE.lobehub.txt')),'Missing icon license'
        def assets():
            assert profile.read_bytes()==b'keep-profile'
            assert skill.read_text(encoding='utf-8')=='keep-skill'
            assert json.loads(settings.read_text(encoding='utf-8'))=={'keep':True}
        child=None;job=None
        try:
            install();payload();assets()
            child=subprocess.Popen([str(folder/'requirement-french.exe')],env=env,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
            job=windows.Job(child)
            start=data/'browser/start.url'
            wait_for(lambda:start.is_file() and bool(start.read_text(encoding='utf-8').strip()))
            page=start.read_text(encoding='utf-8').strip()
            opener=urllib.request.build_opener(urllib.request.ProxyHandler({}))
            with opener.open(page,timeout=15) as response:assert response.status==200
            context=page.rsplit('/',1)[0]+'/context'
            with opener.open(context,timeout=15) as response:
                body=response.read(131072)
                assert b'secret-marker' not in body and b'system_proxy' not in body
                assert isinstance(json.loads(body),dict)
            assert child.poll() is None,'Installed host exited before verification'
            # ----- 实际同资料浏览器进程：仅查询 CI 样本路径，不打印命令行 -----
            query='$p=@(Get-CimInstance Win32_Process -Filter "Name = \'chrome.exe\'" | Where-Object { $_.ExecutablePath -like ($env:CC_INSTALL_DIR+"*") -and $_.CommandLine -like ("*"+$env:CC_PROFILE+"*") }); if($p.Count -eq 0){exit 1}'
            wait_for(lambda:subprocess.run(['powershell','-NoProfile','-Command',query],env=env|{'CC_INSTALL_DIR':str(folder),'CC_PROFILE':str(profile.parent.parent)},stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL).returncode==0)
            job.stop();child.wait(timeout=20);job.close();job=None;child=None
            install();payload();assets()
            uninstallers=list(folder.glob('*uninstall*.exe'))
            assert len(uninstallers)==1,'Expected one application uninstaller'
            subprocess.run(f'"{uninstallers[0]}" /S _?={folder}',check=True,timeout=180,env=env)
            wait_for(lambda:not (folder/'requirement-french.exe').exists())
            assets()
            print('Windows current-user install, A/B launch, upgrade and uninstall verified; user assets preserved.')
        finally:
            if job:job.stop();job.close()
            if child:child.wait(timeout=20)
            shutil.rmtree(data)

if __name__=='__main__':main()
