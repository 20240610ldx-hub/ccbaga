#!/usr/bin/env python3
# [INPUT]: Windows PowerShell 5.1、Rust 样本程序与可丢弃官方安装目录。
# [OUTPUT]: 真正 CLI 卸载、来源保留重装、失败/自定义启动器拒绝与个人资产保留的回归。
# [POS]: Windows 平台行为测试；仅操作临时样本，不卸载 runner 或用户的真实 Claude。
# [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT=Path(__file__).resolve().parent.parent
POWERSHELL=Path(os.environ['SystemRoot'])/'System32/WindowsPowerShell/v1.0/powershell.exe'
SCRIPT=(ROOT/'scripts/claude-cli.ps1').read_text(encoding='utf-8-sig')

class CliTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.build=tempfile.TemporaryDirectory(prefix='ccbaga-cli-binary-')
        source=Path(cls.build.name)/'cli.rs'
        source.write_text(r'''fn main(){
if std::env::current_exe().unwrap().file_stem().unwrap()=="winget" {
let target=std::path::PathBuf::from(std::env::var("LOCALAPPDATA").unwrap()).join("Microsoft/WinGet/Packages/Anthropic.ClaudeCode_Microsoft.Winget.Source_8wekyb3d8bbwe/claude.exe");
match std::env::args().nth(1).as_deref() {
Some("uninstall") => { std::fs::remove_file(target).unwrap(); },
Some("install") => { std::fs::copy(std::env::var("CC_FIXTURE_WINGET_PAYLOAD").unwrap(),target).unwrap(); },
_ => std::process::exit(9)
} return;
} println!("2.1.0 (Claude Code)");
}''',encoding='utf-8')
        cls.binary=source.with_suffix('.exe')
        subprocess.run(['rustc',str(source),'-o',str(cls.binary)],check=True)
    @classmethod
    def tearDownClass(cls):cls.build.cleanup()
    def setUp(self):
        self.temporary=tempfile.TemporaryDirectory(prefix='ccbaga-cli-')
        self.home=Path(self.temporary.name)
        self.bin=self.home/'bin';self.bin.mkdir()
        self.local=self.home/'AppData/Local';self.local.mkdir(parents=True)
        self.skill=self.file('.claude/skills/personal/SKILL.md','keep-skill')
        self.settings=self.file('.claude/settings.json','{"keep":true}')
        self.env=os.environ|{'CC_CLI_USER_DIR':str(self.home),'USERPROFILE':str(self.home),'LOCALAPPDATA':str(self.local),'PATH':str(self.bin)+';'+str(Path(os.environ['SystemRoot'])/'System32'),'CC_FIXTURE_ROOT':str(self.home)}
    def tearDown(self):self.temporary.cleanup()
    def file(self,name,text):
        path=self.home/name;path.parent.mkdir(parents=True,exist_ok=True);path.write_text(text,encoding='utf-8');return path
    def invoke(self,action):
        command="[Console]::InputEncoding=New-Object System.Text.UTF8Encoding; & ([scriptblock]::Create([Console]::In.ReadToEnd())) '"+action+"'"
        return subprocess.run([str(POWERSHELL),'-NoProfile','-NonInteractive','-ExecutionPolicy','Bypass','-Command',command],input=SCRIPT,env=self.env,capture_output=True,encoding='utf-8',timeout=30)
    def assets(self):
        self.assertEqual(self.skill.read_text(encoding='utf-8'),'keep-skill')
        self.assertEqual(json.loads(self.settings.read_text(encoding='utf-8')),{'keep':True})
    def native(self):
        binary=self.home/'.local/bin/claude.exe';binary.parent.mkdir(parents=True)
        version=self.home/'.local/share/claude/versions/2.1.0';version.parent.mkdir(parents=True)
        shutil.copyfile(self.binary,binary);shutil.copyfile(self.binary,version)
        return binary,version
    def npm(self):
        package='node_modules/@anthropic-ai/claude-code/package.json'
        self.file('bin/'+package,'{"name":"@anthropic-ai/claude-code"}')
        self.file('bin/claude.cmd','@echo off\nrem node_modules\\@anthropic-ai\\claude-code\\cli.js\necho 2.1.0 (Claude Code)\n')
        helper=self.file('npm.ps1',r'''[Console]::OutputEncoding=New-Object System.Text.UTF8Encoding
$r=$env:CC_FIXTURE_ROOT
$p=Join-Path $r 'bin\node_modules\@anthropic-ai\claude-code'
switch($args[0]) {
'root' { Write-Output (Join-Path $r 'bin\node_modules') }
'uninstall' { Remove-Item -LiteralPath $p -Recurse -Force; Remove-Item -LiteralPath (Join-Path $r 'bin\claude.cmd') -Force }
'install' { if($env:CC_FIXTURE_FAIL){exit 9}; New-Item -Path $p -ItemType Directory -Force | Out-Null; Set-Content -LiteralPath (Join-Path $p 'package.json') -Value '{"name":"@anthropic-ai/claude-code"}'; Set-Content -LiteralPath (Join-Path $r 'bin\claude.cmd') -Value "@echo off`nrem node_modules\@anthropic-ai\claude-code\cli.js`necho 2.2.0 (Claude Code)" }
default {exit 8}
}
''')
        helper.write_text(helper.read_text(),encoding='utf-8-sig')
        self.file('bin/npm.cmd','@echo off\n"'+str(POWERSHELL)+'" -NoProfile -ExecutionPolicy Bypass -File "%CC_FIXTURE_ROOT%\\npm.ps1" %*\nexit /b %errorlevel%\n')
    def test_native_read_and_real_uninstall_keep_assets(self):
        binary,version=self.native()
        read=self.invoke('read');self.assertEqual(read.returncode,0,read.stderr);self.assertIn('CC_CLI_METHOD=native',read.stdout)
        result=self.invoke('uninstall');self.assertEqual(result.returncode,0,result.stderr);self.assertIn('CC_CLI_DONE=1',result.stdout)
        self.assertFalse(binary.exists());self.assertFalse(version.exists());self.assets()
    def test_npm_reinstall_and_uninstall(self):
        self.npm();result=self.invoke('reinstall')
        self.assertEqual(result.returncode,0,result.stderr);self.assertIn('CC_CLI_METHOD=npm',result.stdout);self.assertIn('CC_CLI_VERSION=2.2.0',result.stdout)
        result=self.invoke('uninstall');self.assertEqual(result.returncode,0,result.stderr);self.assertFalse((self.bin/'claude.cmd').exists());self.assets()
    def test_failed_npm_reinstall_does_not_claim_success(self):
        self.npm();self.env['CC_FIXTURE_FAIL']='1';result=self.invoke('reinstall')
        self.assertNotEqual(result.returncode,0);self.assertNotIn('CC_CLI_DONE=1',result.stdout);self.assets()
    def test_winget_reinstall_preserves_package_manager(self):
        destination=self.local/'Microsoft/WinGet/Packages/Anthropic.ClaudeCode_Microsoft.Winget.Source_8wekyb3d8bbwe/claude.exe'
        destination.parent.mkdir(parents=True);shutil.copyfile(self.binary,destination)
        shutil.copyfile(self.binary,self.bin/'winget.exe')
        self.env['CC_FIXTURE_WINGET_PAYLOAD']=str(self.binary)
        result=self.invoke('reinstall');self.assertEqual(result.returncode,0,result.stderr)
        self.assertIn('CC_CLI_METHOD=winget',result.stdout)
        result=self.invoke('uninstall');self.assertEqual(result.returncode,0,result.stderr)
        self.assertFalse(destination.exists());self.assets()
    def test_custom_launcher_is_preserved(self):
        custom=self.file('bin/claude.cmd','@echo off\necho personal launcher\n')
        result=self.invoke('uninstall');self.assertNotEqual(result.returncode,0);self.assertTrue(custom.exists());self.assets()
    def test_unmanaged_native_launcher_is_preserved_without_path(self):
        native=self.home/'.local/bin/claude.exe';native.parent.mkdir(parents=True);shutil.copyfile(self.binary,native)
        result=self.invoke('reinstall');self.assertNotEqual(result.returncode,0);self.assertTrue(native.exists());self.assets()

class SystemTests(unittest.TestCase):
    @unittest.skipUnless(os.environ.get('GITHUB_ACTIONS')=='true','System mutation is restricted to disposable CI runners')
    def test_language_format_write_retains_languages_and_restores_state(self):
        script=r"""
$ErrorActionPreference='Stop'
$env:PSModulePath=(Join-Path $PSHOME 'Modules')+';'+$env:PSModulePath
[Console]::OutputEncoding=New-Object System.Text.UTF8Encoding
$before=Get-WinUserLanguageList
$override=Get-WinUILanguageOverride
$format=Get-Culture
$original=@($before | ForEach-Object { $_.LanguageTag+':'+($_.InputMethodTips -join ',') })
try {
  $env:CC_REGION_LANGUAGE=$before[0].LanguageTag
  $env:CC_REGION_LOCALE='en-CA'
  & $env:CC_SYSTEM_SCRIPT | Out-Null
  $after=@(Get-WinUserLanguageList | ForEach-Object { $_.LanguageTag+':'+($_.InputMethodTips -join ',') })
  if(Compare-Object $original $after){throw 'Existing languages/input methods changed'}
  $actual=(& $env:CC_POWERSHELL -NoProfile -NonInteractive -Command '(Get-Culture).Name').Trim()
  if($actual -ne 'en-CA'){throw 'Regional format was not persisted'}
} finally {
  Set-WinUserLanguageList $before -Force
  if($override){Set-WinUILanguageOverride -Language $override.Name}else{Set-WinUILanguageOverride}
  Set-Culture -CultureInfo $format.Name
}
Write-Output 'SYSTEM_VERIFIED'
"""
        result=subprocess.run([str(POWERSHELL),'-NoProfile','-NonInteractive','-Command',script],env=os.environ|{'CC_SYSTEM_SCRIPT':str(ROOT/'scripts/windows-region.ps1'),'CC_POWERSHELL':str(POWERSHELL)},capture_output=True,encoding='utf-8',timeout=45)
        self.assertEqual(result.returncode,0,result.stderr);self.assertIn('SYSTEM_VERIFIED',result.stdout)

if __name__=='__main__':unittest.main()
