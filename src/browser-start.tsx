/**
 * [INPUT]: 随机令牌回环服务上下文、浏览器原生 navigator/Intl/Permissions 与实际 HTTPS 出口与服务备援目录与宿主界面语言。
 * [OUTPUT]: 实测起始页，同步宿主中英文界面，显示本机通讯状态与回报时间，观测上报宿主；逐项原因和实际/目标对比，检测中黄底波纹、失败红底、完整通过绿底大对勾。
 * [POS]: 独立 Chromium 入口，复用工具箱控件；成功状态只由真实观测决定，不接入 Tauri 权限，不伪造地理位置。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { Button, MotionRoot, TooltipProvider } from 'cromma-ui';
import { useEffect, useState } from 'react';
import {setLocale,useI18n} from "./i18n";
import { createRoot } from 'react-dom/client';
import { type BrowserContext, type Report, type Exit } from './browser-model';
import { probeBrowserExit } from './browser-network';
import {verifyBrowser} from './browser-verification';
import browserLogo from '../assets/browser-logo.jpg';
import './index.css';
import './style.css';
async function api<T>(route: string, body?: unknown): Promise<T> { const response = await fetch(new URL(route, location.href), { signal: AbortSignal.timeout(3000), cache:'no-store', credentials:'omit', redirect:'error', ...(body ? { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(body) } : {}) }); if (!response.ok)
    throw new Error("本地核验连接失败，请从CCBaga-A重新打开浏览器。"); return response.json(); }
function Start() {
    const {t,locale}=useI18n();
    const [context, setContext] = useState<BrowserContext | null>(null), [report, setReport] = useState<Report | null>(null), [busy, setBusy] = useState(true), [error, setError] = useState(''), [linked, setLinked] = useState(false), [reportedAt, setReportedAt] = useState(0);
    async function measure() {
        setBusy(true);
        setError('');
        setReport(null);
        setReportedAt(0);
        try {
            const context = await api<BrowserContext>('context');
            setContext(context);setLocale(context.ui_language);
            setLinked(true);
            const ua = (navigator as Navigator & {
                userAgentData?: {
                    getHighEntropyValues: (keys: string[]) => Promise<{
                        platformVersion?: string;
                        uaFullVersion?: string;
                    }>;
                };
            }).userAgentData;
            const high = await ua?.getHighEntropyValues(['platformVersion', 'uaFullVersion']).catch(() => null);
            const permission = await navigator.permissions.query({ name: 'geolocation' }).then(x => x.state).catch(() => 'unknown');
            let exit: Exit | null = null, networkError = '';
            try {
                exit = await probeBrowserExit(context.exit_services);
            }
            catch (e) {
                networkError = e instanceof Error ? e.message : String(e);
            }
            const observed: Report = { language: navigator.language, languages: [...navigator.languages], timezone: Intl.DateTimeFormat().resolvedOptions().timeZone, platform: navigator.platform, cores: navigator.hardwareConcurrency, user_agent: navigator.userAgent, platform_version: high?.platformVersion || '', browser_version: high?.uaFullVersion || navigator.userAgent.match(/(?:Chrome|Chromium)\/([\d.]+)/)?.[1] || '', geolocation: permission, exit, error: networkError };
            await api('report', observed);
            setReport(observed);
            setReportedAt(Date.now());
            setLinked(true);
        }
        catch (e) {
            setLinked(false);
            setError(String(e));
        }
        finally {
            setBusy(false);
        }
    }
    useEffect(() => { void measure(); let live = true; const timer = setInterval(() => { void api<BrowserContext>('context').then(next => { if (live) {
        setContext(next);setLocale(next.ui_language);
        setLinked(true);
    } }).catch(() => { if (live)
        setLinked(false); }); }, 10000); return () => { live = false; clearInterval(timer); }; }, []);
    const result=verifyBrowser(context,report,linked,error,busy);
    const {phase,checks,problems,failures}=result;
    const title=phase==='checking'?'检测中':phase==='passed'?'检测通过':'检测未通过';
    return <div className={`browser-verification-page is-${phase}`} data-verification={phase}>
     <main className="browser-start">
      <header className="verification-brand"><div><img src={browserLogo} width={32} height={32} alt=""/><strong>CCBaga-B</strong></div><span>Chromium {context?.engine_version||'—'}</span></header>
      <section className="verification-banner" aria-labelledby="verification-title" aria-busy={busy}>
       <div className="verification-signal" aria-hidden="true"><i/><i/><svg viewBox="0 0 96 96"><circle cx="48" cy="48" r="44" fill="currentColor"/>{phase==='passed'?<path d="m27 49 14 14 28-31"/>:phase==='failed'?<path d="m34 34 28 28m0-28L34 62"/>:<><path d="M48 27v23"/><circle cx="48" cy="65" r="3" fill="white" stroke="none"/></>}</svg></div>
       <div className="verification-title" aria-live="polite"><span>{t('浏览器环境')}</span><h1 id="verification-title">{t(title)}</h1>{!busy&&<span>{t('已通过 {v0} / {v1} 项',{v0:checks.filter(check=>check.ok).length,v1:checks.length})}</span>}</div>
       <Button variant="ink" loading={busy} disabled={busy} onClick={()=>void measure()}>{t('重新实测')}</Button>
      </section>
      <div className="landing-section-band" aria-hidden="true"/>
      {phase==='failed'&&<section className="verification-reasons" aria-labelledby="verification-reasons-title"><h2 id="verification-reasons-title">{t('未通过原因')}</h2><ul>{problems.map(problem=><li key={problem}>{t(problem)}</li>)}{failures.map(check=><li key={check.label}><strong>{t(check.label)}</strong><span>{t(check.actual)} · {t('目标：{v0}',{v0:t(check.expected)})}</span></li>)}</ul></section>}
      {checks.length>0&&<section className="verification-results" aria-label={t('逐项检测结果')}>{checks.map(check=><article className={`verification-check ${check.ok?'ok':'failed'}`} key={check.label}><header><h2>{t(check.label)}</h2><span aria-label={t(check.ok?'检测通过':'检测未通过')}>{check.ok?'✓':'×'}</span></header><strong>{t(check.actual)}</strong>{!check.ok&&<p>{t('目标：{v0}',{v0:t(check.expected)})}</p>}</article>)}</section>}
      {phase==='checking'&&<div className="verification-loading" role="status">{t('正在读取浏览器与网络…')}</div>}
      {report?.exit&&<section className="verification-network"><div><span>{t('出口 IP')}</span><strong>{report.exit.ip}</strong></div><div><span>{t('国家 / 城市')}</span><strong>{report.exit.country} / {report.exit.city||'—'}</strong></div></section>}
      <footer className="verification-footer"><span className={linked?'connected':'disconnected'}>{t(linked?'已连接 CCBaga-A':'主应用未连接')}</span><time>{reportedAt?t('结果回传成功 {v0}',{v0:new Date(reportedAt).toLocaleTimeString(locale==='zh'?'zh-CN':'en-US')}):t('等待检测回传')}</time></footer>
     </main>
    </div>;

}
createRoot(document.getElementById('app')!).render(<MotionRoot><TooltipProvider><Start /></TooltipProvider></MotionRoot>);
