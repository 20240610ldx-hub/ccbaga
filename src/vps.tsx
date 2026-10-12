/**
 * [INPUT]: 固定资源入口、三个 AI 品牌图、Cromma Tabs/Textarea、YAML 安装提示词与原生剪贴板。
 * [OUTPUT]: VPS/家宽/Skill 三步入口、AI 图标分段与随语言切换的一键复制安装提示词。
 * [POS]: 网络资源导航；应用不收集密钥或执行远端部署，Skill 在用户选定的 AI 工具内运行。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import {Button,Tabs,Textarea} from 'cromma-ui';
import {useState} from 'react';
import {invoke} from '@tauri-apps/api/core';
import {useI18n} from './i18n';
import {agents,installationPrompt} from './network-skill';
export function VpsPanel() {
    const {t,locale}=useI18n();
    const [selected,setSelected]=useState('claude'), [copied,setCopied]=useState(''), [error,setError]=useState('');
    const prompt=installationPrompt(selected,locale);
    async function open(provider:'dmit'|'webshare'|'skill') {
        setError('');
        try { await invoke('network_open_resource',{provider}); }
        catch (e) { setError(String(e)); }
    }
    async function copy() {
        setError('');
        try { await invoke('copy_result',{text:prompt});setCopied(prompt); }
        catch(e) { setError(String(e)); }
    }
    return <section className="system-page network-page"><div className="cleanup-title"><h1>{t('网络环境')}</h1></div>
        <div className="network-steps">{([
            ['dmit','获取 VPS','info'],['webshare','获取家宽','success'],['skill','安装 Skill','secondary']
        ] as const).map(([provider,label,variant],index)=><div className="network-step" key={provider}><span className="network-step-number" aria-hidden="true">{index+1}</span><Button variant={variant} onClick={()=>void open(provider)}>{t(label)}</Button></div>)}</div>
        <div className="skill-install"><Tabs variant="segmented" value={selected} onValueChange={setSelected} tabs={agents.map(agent=>({value:agent.id,label:<span className="coding-agent"><img src={agent.icon} width={20} height={20} alt=""/>{agent.name}</span>}))}/>
            <label className="skill-prompt-label" htmlFor="skill-prompt">{t('安装提示词')}</label>
            <Textarea id="skill-prompt" readOnly value={prompt} rows={7}/>
            <Button variant="ink" onClick={()=>void copy()}>{copied===prompt?t('已复制'):t('复制提示词')}</Button>
        </div>
        {error&&<p role="alert" className="system-feedback error">{t(error)}</p>}
    </section>;
}
