/**
 * [INPUT]: 跨平台虚构宿主上下文和网页观测。
 * [OUTPUT]: 红/黄/绿状态、失败原因、断连/未知值与目标时区变化回归。
 * [POS]: 真实页面与纯核验模型的契约测试，不调用原生权限或外部服务。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import {test,expect} from 'bun:test';
import {verifyBrowser} from './browser-verification';
import type {BrowserContext,Report} from './browser-model';
const context:BrowserContext={ui_language:'zh',exit_services:[],region:{id:'us-la',label:'LA',country:'US',language:'en-US',locale:'en_US',timezone:'America/Los_Angeles'},config:{seed:1,proxy:'',follow_machine:false,language:'en-US',timezone:'America/Los_Angeles',native_graphics:true},machine:{platform:'windows',version:'10.0.26100',cores:8,timezone:'America/Los_Angeles',languages:['en-US'],locale:'en_US',location_enabled:false,timezones:[]},engine_version:'148.0.7778.215'};
const report:Report={language:'en-US',languages:['en-US'],timezone:'America/Los_Angeles',platform:'Win32',platform_version:'10.0.26100',cores:8,user_agent:'fixture',browser_version:'148.0.7778.215',geolocation:'denied',exit:{ip:'203.0.113.1',country:'US',city:'LA',timezone:'America/Los_Angeles',isp:'fixture'},error:''};
test('只有全部实测匹配且回传连接有效时才变绿，重测立即变黄',()=>{
 expect(verifyBrowser(context,report,true,'',false).phase).toBe('passed');
 expect(verifyBrowser(context,report,true,'',true).phase).toBe('checking');
 expect(verifyBrowser(context,report,false,'',false).phase).toBe('failed');
 expect(verifyBrowser(context,report,true,'report failed',false).problems).toContain('report failed');
 expect(verifyBrowser(null,null,false,'',false).phase).toBe('failed');
});
test('所有失败都有具体值或原因，未知出口与定位不能显示成功',()=>{
 const result=verifyBrowser({...context,machine:{...context.machine,location_enabled:null}},{...report,cores:4,exit:null,error:'HTTP 429'},true,'',false);
 expect(result.phase).toBe('failed');expect(result.problems).toContain('HTTP 429');
 expect(result.failures.find(row=>row.label==='逻辑 CPU')).toMatchObject({actual:'4',expected:'8'});
 expect(result.failures.find(row=>row.label==='本机定位')?.actual).toBe('尚未核验');
 expect(verifyBrowser({...context,config:{...context.config,timezone:'America/New_York'}},report,true,'',false).phase).toBe('failed');
 for(const [platform,value] of [['macos','MacIntel'],['linux','Linux x86_64']] as const){
  const ctx={...context,machine:{...context.machine,platform}};
  const observed={...report,platform:value};
  expect(verifyBrowser(ctx,observed,true,'',false).phase).toBe('passed');
 }
});
