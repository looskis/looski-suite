// Optional real-Chrome verification; uses only Node's built-in CDP/WebSocket support.
import { spawn, execFileSync } from 'node:child_process';
import { mkdtempSync, readFileSync, existsSync, rmSync, mkdirSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import assert from 'node:assert/strict';
const state=JSON.parse(execFileSync('python3',['-c',`import sys,json,secrets
sys.path.insert(0,'scripts')
from integration_fixture import Fixture
f=Fixture();base='/api/v1/workspaces/'+f.workspace
module=next(m for m in f.call(base+'/modules') if m['key']=='helpdesk')
if not module['enabled']:f.call(base+'/modules/helpdesk','PATCH',{'enabled':True,'expected_version':module['version']},key=secrets.token_hex(16))
s=f.call(base+'/helpdesk/settings')
if not s['accepting']:s=f.call(base+'/helpdesk/settings','PUT',{'accepting':True,'expected_version':s['version']},key=secrets.token_hex(16))
print(json.dumps({'url':f.base,'workspace':f.workspace,'token':f.token,'publicKey':s['public_key'],'enabledBefore':module['enabled']}))`],{encoding:'utf8'}));
const profile=mkdtempSync(join(tmpdir(),'looski-helpdesk-chrome-'));
const chrome=spawn(process.env.CHROME_PATH??'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',['--headless=new','--disable-gpu','--no-first-run','--no-default-browser-check','--remote-debugging-port=0',`--user-data-dir=${profile}`,'about:blank'],{stdio:'ignore'});
const delay=ms=>new Promise(r=>setTimeout(r,ms));
let ws;let sequence=0;const pending=new Map();let dialogs=0;
function call(method,params={}){return new Promise((resolve,reject)=>{const id=++sequence;const timeout=setTimeout(()=>{pending.delete(id);reject(new Error('CDP timeout: '+method));},15000);pending.set(id,{resolve,reject,timeout});ws.send(JSON.stringify({id,method,params}));});}
async function evaluate(expression){const r=await call('Runtime.evaluate',{expression,returnByValue:true,awaitPromise:true});if(r.exceptionDetails)throw new Error(r.exceptionDetails.exception?.description??'Browser expression failed');return r.result.value;}
async function until(expression){for(let i=0;i<100;i++){if(await evaluate(expression))return;await delay(100);}throw new Error('Browser condition timed out');}
async function navigate(url){await call('Page.navigate',{url});await until(`document.readyState==='complete'&&location.href===${JSON.stringify(url)}`);await delay(150);}
async function submit(action,fields){await evaluate(`(()=>{const f=[...document.forms].find(f=>(f.getAttribute('action')??'')===${JSON.stringify(action)});for(const [name,value] of Object.entries(${JSON.stringify(fields)})){const el=f.elements.namedItem(name);el.value=value;el.dispatchEvent(new Event('input',{bubbles:true}));el.dispatchEvent(new Event('change',{bubbles:true}));}f.requestSubmit();return true;})()`);await delay(250);await until(`document.readyState==='complete'`);}
const artifacts=join(profile,'screenshots');mkdirSync(artifacts);
try{
 for(let i=0;i<100&&!existsSync(join(profile,'DevToolsActivePort'));i++)await delay(100);
 const port=readFileSync(join(profile,'DevToolsActivePort'),'utf8').split('\n')[0];
 const page=await(await fetch(`http://127.0.0.1:${port}/json/new?about:blank`,{method:'PUT'})).json();
 ws=new WebSocket(page.webSocketDebuggerUrl);await new Promise((resolve,reject)=>{ws.onopen=resolve;ws.onerror=reject;});
 ws.onmessage=event=>{const m=JSON.parse(event.data);if(m.method==='Page.javascriptDialogOpening')dialogs++;const p=pending.get(m.id);if(p){clearTimeout(p.timeout);pending.delete(m.id);m.error?p.reject(new Error(m.error.message)):p.resolve(m.result);}};
 await call('Page.enable');await call('Runtime.enable');await call('Network.enable');await call('Emulation.setDeviceMetricsOverride',{width:1365,height:1000,deviceScaleFactor:1,mobile:false});
 await navigate(state.url+'/support/new/'+state.publicKey);
 const payload='<img src=x onerror="window.helpdeskInjected=1"><script>window.helpdeskInjected=1</script>';
 await submit('',{requester_name:'Chrome requester',subject:'Chrome support '+payload,body:'Browser question '+payload});
 await until(`document.body.innerText.includes('Your ticket is open')`);
 const ticketPath=await evaluate(`document.querySelector('a[href^="/support/t/"]').getAttribute('href')`);
 await navigate(state.url+ticketPath);
 assert.equal(await evaluate('Boolean(window.helpdeskInjected)'),false);assert.equal(await evaluate('document.querySelectorAll("img[src=x]").length'),0);
 await submit('?/reply',{body:'Chrome customer reply '+payload});await until(`document.body.innerText.includes('Chrome customer reply')`);
 const ticket=ticketPath.split('/').at(-1);
 await call('Network.setCookie',{name:'looski_access',value:state.token,url:state.url,path:'/',httpOnly:true,sameSite:'Lax'});
 const staff=state.url+'/workspaces/'+state.workspace+'/helpdesk/'+ticket;
 await navigate(staff);await until(`document.body.innerText.includes('Conversation')`);
 await submit('?/reply',{body:'Private Chrome note '+payload,internal:'true'});await until(`document.body.innerText.includes('Private Chrome note')`);
 assert.equal(await evaluate('Boolean(window.helpdeskInjected)'),false);assert.equal(dialogs,0);
 // Screenshots are local review artifacts, never part of the application or source tree.
 const screenshot=await call('Page.captureScreenshot',{format:'png',captureBeyondViewport:false});
 const {writeFileSync}=await import('node:fs');writeFileSync('/tmp/looski-helpdesk-staff.png',Buffer.from(screenshot.data,'base64'));
 await navigate(state.url+ticketPath);assert.equal(await evaluate(`document.body.innerText.includes('Private Chrome note')`),false);
 await call('Emulation.setDeviceMetricsOverride',{width:390,height:844,deviceScaleFactor:1,mobile:true});await delay(150);
 assert.equal(await evaluate('window.innerWidth<=390 && document.documentElement.scrollWidth<=390'),true);
 const mobile=await call('Page.captureScreenshot',{format:'png'});writeFileSync('/tmp/looski-helpdesk-customer-mobile.png',Buffer.from(mobile.data,'base64'));
 assert.equal(dialogs,0);console.log('PASS: real Chrome submissions, inert stored markup, staff/private separation and mobile overflow; screenshots in /tmp/looski-helpdesk-*.png');
}finally{
 ws?.close();chrome.kill();await delay(300);rmSync(profile,{recursive:true,force:true});
 if(!state.enabledBefore){const headers={'Authorization':'Bearer '+state.token,'Content-Type':'application/json'};const base=state.url+'/api/v1/workspaces/'+state.workspace;const modules=await(await fetch(base+'/modules',{headers})).json();const m=modules.find(m=>m.key==='helpdesk');await fetch(base+'/modules/helpdesk',{method:'PATCH',headers:{...headers,'Idempotency-Key':crypto.randomUUID()},body:JSON.stringify({enabled:false,expected_version:m.version})});}
}
