/**
 * [INPUT]: 现有隔离 Playwright 引擎、Vite 本机地址与虚构出口/宿主响应。
 * [OUTPUT]: 真实 browser.html 的黄波纹/绿通过/红失败、重测与双语 DOM 回归。
 * [POS]: 身份 UI 回归共用引擎的伴生页验证，不截图、不调用真实宿主或外网。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import {strict as assert} from 'node:assert';
import type {Browser} from 'playwright-core';
export async function testBrowserVerification(browser:Browser,base:string) {
 const context=await browser.newContext({locale:'en-US',timezoneId:'America/Los_Angeles'});
 await context.addInitScript(()=>{
  const query=navigator.permissions.query.bind(navigator.permissions);
  navigator.permissions.query=(options:PermissionDescriptor)=>options.name==='geolocation'?Promise.resolve({state:'denied'} as PermissionStatus):query(options);
 });
 const page=await context.newPage();page.setDefaultTimeout(10000);
 let release!:()=>void;const gate=new Promise<void>(resolve=>{release=resolve;});
 let first=true, failure=false, language='zh', reports=0;
 await context.route('**/*',async route=>{
  const url=route.request().url();
  if(!url.startsWith(base+'/'))return route.abort();
  if(url===base+'/context'){
   if(first){first=false;await gate;}
   const observed=await page.evaluate(async()=>{
    const high=await (navigator as any).userAgentData?.getHighEntropyValues(['platformVersion','uaFullVersion']);
    return {platform:navigator.platform.toLowerCase().includes('mac')?'macos':navigator.platform==='Win32'?'windows':'linux',version:high?.platformVersion||'',cores:navigator.hardwareConcurrency,engine:high?.uaFullVersion||navigator.userAgent.match(/(?:Chrome|Chromium)\/([\d.]+)/)?.[1]||''};
   });
   return route.fulfill({json:{ui_language:language,exit_services:[base+'/fixture-exit'],region:{id:'us-la',country:'US',language:'en-US',locale:'en_US',timezone:'America/Los_Angeles'},config:{follow_machine:false,timezone:'America/Los_Angeles'},machine:{platform:observed.platform,version:observed.version,cores:observed.cores,timezone:'America/Los_Angeles',languages:['en-US'],locale:'en_US',location_enabled:false},engine_version:observed.engine}});
  }
  if(url===base+'/fixture-exit')return route.fulfill({json:{ip:'203.0.113.1',country_code:failure?'JP':'US',city:'Fixture',timezone:'America/Los_Angeles'}});
  if(url===base+'/report'){reports++;return route.fulfill({json:{ok:true}});}
  return route.continue();
 });
 try{
  await page.goto(base+'/browser.html');
  await page.waitForFunction(()=>document.querySelector('[data-verification]')?.getAttribute('data-verification')==='checking');
  assert.equal(await page.locator('.verification-signal i').first().evaluate(node=>getComputedStyle(node).animationName),'verification-ripple');
  release();
  await page.waitForFunction(()=>document.querySelector('[data-verification]')?.getAttribute('data-verification')==='passed');
  assert.equal(await page.locator('.verification-check.failed').count(),0);assert(reports>0);
  await page.waitForFunction(()=>getComputedStyle(document.querySelector('[data-verification]')!).backgroundColor==='rgb(228, 245, 233)');
  failure=true;
  await page.getByRole('button',{name:'重新实测',exact:true}).click();
  await page.waitForFunction(()=>document.querySelector('[data-verification]')?.getAttribute('data-verification')==='failed');
  assert.match(await page.locator('.verification-reasons').innerText(),/出口国家/);
  assert.match(await page.locator('.verification-reasons').innerText(),/JP.*US/);
  await page.waitForFunction(()=>getComputedStyle(document.querySelector('[data-verification]')!).backgroundColor==='rgb(255, 241, 240)');
  failure=false;language='en';
  await page.getByRole('button',{name:'重新实测',exact:true}).click();
  await page.getByRole('heading',{name:'Checks passed',exact:true}).waitFor();
  await page.getByRole('button',{name:'Run check',exact:true}).waitFor();
  return 4;
 } finally {release();await context.close();}
}
