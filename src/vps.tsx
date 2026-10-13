/**
 * [INPUT]: 固定资源入口、服务规格、Claude 核心域名分流诊断、Cromma Tabs/Textarea、双语提示词与原生剪贴板。
 * [OUTPUT]: Laper 满宽细线三步入口与斜纹分隔、核心域名分流诊断列表、五项 AI 图标分段与双语提示词一键复制。
 * [POS]: 网络资源与分流诊断；应用不收集密钥或执行远端部署，诊断仅进行只读网络连通性探测。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import {Button,Tabs,Textarea} from 'cromma-ui';
import {Server as ServerIcon,HomeDefault as HomeIcon,Refresh} from 'cromma-ui/pika';
import {useState} from 'react';
import {invoke} from '@tauri-apps/api/core';
import {useI18n} from './i18n';
import {AppDialog} from './dialog';
import {agents,installationPrompt} from './network-skill';

export type DomainDiagnosis = {
    domain: string;
    ok: boolean;
    status_code: number | null;
    latency_ms: number | null;
    ip: string | null;
    message: string;
};

export function VpsPanel() {
    const {t,locale}=useI18n();
    const [selected,setSelected]=useState('generic'), [copied,setCopied]=useState(''), [error,setError]=useState('');
    const [guide,setGuide]=useState<'dmit'|'webshare'|null>(null);
    const [diagnosing,setDiagnosing]=useState(false);
    const [diagnostics,setDiagnostics]=useState<DomainDiagnosis[]|null>(null);
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

    async function runDiagnostics() {
        setDiagnosing(true);
        setError('');
        try {
            const results = await invoke<DomainDiagnosis[]>('network_diagnose_domains');
            setDiagnostics(results);
        } catch (e) {
            setError(String(e));
        } finally {
            setDiagnosing(false);
        }
    }

    return <section className="system-page network-page"><div className="cleanup-title"><h1>{t('网络环境')}</h1></div>
        <ol className="network-steps">{([
            ['dmit','DMIT','获取 VPS'],['webshare','Webshare','获取家宽'],['skill','Skill','安装 Skill']
        ] as const).map(([provider,name,label],index)=><li className="network-step" key={provider}>
            <div className="network-step-heading"><span className="network-step-number" aria-hidden="true">{String(index+1).padStart(2,'0')}</span><h2>{name}</h2></div>
            <div className="network-step-actions"><Button variant={provider==='dmit'?'info':provider==='webshare'?'success':'secondary'} size="small" leftIcon={provider==='dmit'?<ServerIcon size={15} aria-hidden="true"/>:provider==='webshare'?<HomeIcon size={15} aria-hidden="true"/>:undefined} onClick={()=>void open(provider)}>{t(label)}</Button>{provider!=='skill'&&<Button variant="secondary" size="small" round aria-label={t(provider==='dmit'?'VPS 帮助':'家宽帮助')} onClick={()=>setGuide(provider)}>?</Button>}</div>
        </li>)}</ol>
        <div className="landing-section-band" aria-hidden="true"/>
        <section className="domain-diagnostics" aria-labelledby="diagnostics-title">
            <div className="domain-diagnostics-header">
                <div className="domain-diagnostics-title-group">
                    <h2 id="diagnostics-title">{t('Claude 核心域名分流诊断')}</h2>
                    <p className="domain-diagnostics-note">{t('只读检测核心服务连通性与分流状态，不携带凭据或私密数据。')}</p>
                </div>
                <Button variant="secondary" size="small" disabled={diagnosing} leftIcon={<Refresh aria-hidden size={15} className={diagnosing?'spin':''} />} onClick={()=>void runDiagnostics()}>{diagnosing?t('诊断中'):diagnostics?t('重新诊断'):t('开始诊断')}</Button>
            </div>
            {diagnostics&&<ul className="domain-diagnostics-list">{diagnostics.map(item=><li key={item.domain} className={'domain-diagnostics-item ' + (item.ok?'ok':'fail')}>
                <div className="domain-item-info">
                    <span className="domain-name">{item.domain}</span>
                    {item.ip&&<span className="domain-ip">{item.ip}</span>}
                </div>
                <div className="domain-item-status">
                    {item.latency_ms!==null&&<span className="domain-latency">{item.latency_ms + ' ms'}</span>}
                    <span className={'domain-badge ' + (item.ok?'badge-ok':'badge-fail')}>{item.status_code?('HTTP ' + item.status_code + ' · ' + t(item.message)):t(item.message)}</span>
                </div>
            </li>)}</ul>}
        </section>
        <div className="landing-section-band" aria-hidden="true"/>
        <section className="skill-install" aria-labelledby="skill-install-title">
            <h2 id="skill-install-title">{t('Skill')}</h2>
            <Tabs className="coding-tabs" size="small" variant="segmented" value={selected} onValueChange={setSelected} tabs={agents.map(agent=>({value:agent.id,label:<span className="coding-agent"><img src={agent.icon} width={18} height={18} alt=""/>{agent.id==='generic'?t('通用'):agent.name}</span>}))}/>
            <div className="skill-prompt-surface"><div className="skill-prompt-toolbar"><span>{selected==='generic'?t('通用'):agents.find(agent=>agent.id===selected)?.name}</span><span>{locale.toUpperCase()}</span></div>
                <Textarea id="skill-prompt" aria-labelledby="skill-install-title" readOnly value={prompt} rows={3}/>
                <div className="skill-prompt-actions"><Button variant="ink" onClick={()=>void copy()}>{copied===prompt?t('已复制'):t('复制提示词')}</Button></div>
            </div>
        </section>
        <AppDialog open={guide!==null} onOpenChange={value=>{if(!value)setGuide(null);}} title={guide==='dmit'?'DMIT':'Webshare'} width={440} footer={<Button variant="ink" onClick={()=>setGuide(null)}>{t('关闭')}</Button>}>
            <dl className="resource-specs">{(guide==='dmit'?[
                ['系统','Debian 12 / 13'],['地址','IPv4'],['登录','SSH Key · root:22'],['文件','PEM · PPK · PUB'],['密钥选项','Generate SSH Key after order']
            ]:[
                ['类型','Static Residential (ISP)'],['连接','Direct Connection'],['认证','Username / Password'],['协议','SOCKS5'],['端口','以面板为准']
            ]).map(([label,value])=><div key={label}><dt>{t(label)}</dt><dd>{t(value)}</dd></div>)}</dl>
        </AppDialog>
        {error&&<p role="alert" className="system-feedback error">{t(error)}</p>}
    </section>;
}
