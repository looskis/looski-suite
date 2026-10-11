// Optional real-Chrome verification; uses only Node's built-in CDP/WebSocket support.
import { spawn, execFileSync } from 'node:child_process';
import { mkdtempSync, readFileSync, existsSync, rmSync, mkdirSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import assert from 'node:assert/strict';
const state=JSON.parse(execFileSync('python3',['-c',`import sys,json,secrets,datetime
sys.path.insert(0,'scripts')
from integration_fixture import Fixture
f=Fixture();base='/api/v1/workspaces/'+f.workspace
initial={m['key']:m['enabled'] for m in f.call(base+'/modules') if m['key'] in ['time','scheduling','forms']}
for k,enabled in initial.items():
 if not enabled:
  m=next(m for m in f.call(base+'/modules') if m['key']==k)
  f.call(base+'/modules/'+k,'PATCH',{'enabled':True,'expected_version':m['version']},key=secrets.token_hex(16))
form=f.call(base+'/forms','POST',{'name':'Chrome intake','draft':{'fields':[{'key':'detail','label':'Preparation notes','kind':'long_text','required':True}]}},expected=201,key=secrets.token_hex(16))
f.call(base+'/forms/'+form['id']+'/publish','POST',{'expected_version':form['version']},key=secrets.token_hex(16))
day=datetime.datetime.now(datetime.timezone.utc).date()+datetime.timedelta(days=1)
while day.isoweekday()!=1:day+=datetime.timedelta(days=1)
print(json.dumps({'url':f.base,'workspace':f.workspace,'token':f.token,'formId':form['id'],'initial':initial,'date':day.isoformat()}))`],{encoding:'utf8'}));
const profile=mkdtempSync(join(tmpdir(),'looski-remaining-chrome-'));
const chrome=spawn(process.env.CHROME_PATH??'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',['--headless=new','--disable-gpu','--no-first-run','--no-default-browser-check','--remote-debugging-port=0',`--user-data-dir=${profile}`,'about:blank'],{stdio:'ignore'});
const delay=ms=>new Promise(r=>setTimeout(r,ms));
let ws;let sequence=0;const pending=new Map();let dialogs=0;
function call(method,params={}){return new Promise((resolve,reject)=>{const id=++sequence;const timeout=setTimeout(()=>{pending.delete(id);reject(new Error('CDP timeout: '+method));},15000);pending.set(id,{resolve,reject,timeout});ws.send(JSON.stringify({id,method,params}));});}
async function evaluate(expression){const r=await call('Runtime.evaluate',{expression,returnByValue:true,awaitPromise:true});if(r.exceptionDetails)throw new Error(r.exceptionDetails.exception?.description??'Browser expression failed');return r.result.value;}
async function until(expression){for(let i=0;i<100;i++){if(await evaluate(expression))return;await delay(100);}throw new Error('Browser condition timed out');}
async function navigate(url){await call('Page.navigate',{url});await until(`document.readyState==='complete'&&location.href===${JSON.stringify(url)}`);await delay(150);}
async function submit(action,fields){await evaluate(`(()=>{const f=[...document.forms].find(f=>(f.getAttribute('action')??'')===${JSON.stringify(action)}&&f.method==='post');for(const [name,value] of Object.entries(${JSON.stringify(fields)})){const el=f.elements.namedItem(name);if(el.type==='checkbox')el.checked=Boolean(value);else el.value=value;el.dispatchEvent(new Event('input',{bubbles:true}));el.dispatchEvent(new Event('change',{bubbles:true}));}f.requestSubmit();return true;})()`);await delay(250);await until(`document.readyState==='complete'`);}
const artifacts=join(profile,'screenshots');mkdirSync(artifacts);
try{
 for(let i=0;i<100&&!existsSync(join(profile,'DevToolsActivePort'));i++)await delay(100);
 const port=readFileSync(join(profile,'DevToolsActivePort'),'utf8').split('\n')[0];
 const page=await(await fetch(`http://127.0.0.1:${port}/json/new?about:blank`,{method:'PUT'})).json();
 ws=new WebSocket(page.webSocketDebuggerUrl);await new Promise((resolve,reject)=>{ws.onopen=resolve;ws.onerror=reject;});
 ws.onmessage=event=>{const m=JSON.parse(event.data);if(m.method==='Page.javascriptDialogOpening')dialogs++;const p=pending.get(m.id);if(p){clearTimeout(p.timeout);pending.delete(m.id);m.error?p.reject(new Error(m.error.message)):p.resolve(m.result);}};
 await call('Page.enable');await call('Runtime.enable');await call('Network.enable');await call('Emulation.setDeviceMetricsOverride',{width:1365,height:1000,deviceScaleFactor:1,mobile:false});
 await call('Network.setCookie',{name:'looski_access',value:state.token,url:state.url,path:'/',httpOnly:true,sameSite:'Lax'});
 const root=state.url+'/workspaces/'+state.workspace;
 const payload='<img src=x onerror="window.bookingInjected=1">';
 await navigate(root+'/time');await until(`document.body.innerText.includes('Record time')`);
 await submit('?/save',{description:'Chrome tracked work '+payload,work_on:state.date,minutes:'4.25'});
 await until(`document.body.innerText.includes('Chrome tracked work')`);
 assert.equal(await evaluate('Boolean(window.bookingInjected)'),false);
 const {writeFileSync}=await import('node:fs');let shot=await call('Page.captureScreenshot',{format:'png'});writeFileSync('/tmp/looski-time-staff.png',Buffer.from(shot.data,'base64'));
 await navigate(root+'/scheduling');await until(`document.body.innerText.includes('Weekly availability')`);
 await submit('?/save',{name:'Chrome appointment',description:'A browser-created appointment',timezone:'UTC',form_id:state.formId,accepting:true});
 await until(`location.search.includes('type=')`);
 const typeId=await evaluate(`new URLSearchParams(location.search).get('type')`);
 const type=await(await fetch(state.url+'/api/v1/workspaces/'+state.workspace+'/scheduling/types/'+typeId,{headers:{Authorization:'Bearer '+state.token}})).json();
 await navigate(state.url+'/book/'+type.public_key+'?date='+state.date);await until(`document.querySelector('select[name=starts_at]')?.options.length>1`);
 const slot=await evaluate(`document.querySelector('select[name=starts_at]').options[1].value`);
 await submit('',{starts_at:slot,attendee_name:'Chrome attendee '+payload,attendee_email:'chrome@example.invalid','answer.detail':'Preparation '+payload});
 await until(`document.body.innerText.includes('Your appointment is confirmed')`);
 const bookingPath=await evaluate(`document.querySelector('a[href^="/appointments/"]').getAttribute('href')`);
 await navigate(state.url+bookingPath+'?date='+state.date);await until(`document.body.innerText.includes('Cancel appointment')`);
 const newSlot=await evaluate(`document.querySelector('select[name=starts_at]').options[0].value`);
 await submit('?/change',{starts_at:newSlot});await until(`!location.search`);
 const bookingId=bookingPath.split('/').at(-1);
 await navigate(root+'/scheduling/'+bookingId);await until(`document.body.innerText.includes('Preparation')`);
 assert.equal(await evaluate('Boolean(window.bookingInjected)'),false);assert.equal(await evaluate('document.querySelectorAll("img[src=x]").length'),0);
 shot=await call('Page.captureScreenshot',{format:'png'});writeFileSync('/tmp/looski-scheduling-staff.png',Buffer.from(shot.data,'base64'));
 await navigate(state.url+bookingPath+'?date='+state.date);
 await call('Emulation.setDeviceMetricsOverride',{width:390,height:844,deviceScaleFactor:1,mobile:true});await delay(150);
 assert.equal(await evaluate('window.innerWidth<=390&&document.documentElement.scrollWidth<=390'),true);
 shot=await call('Page.captureScreenshot',{format:'png'});writeFileSync('/tmp/looski-booking-mobile.png',Buffer.from(shot.data,'base64'));
 await evaluate(`(()=>{const button=document.querySelector('button[value=cancel]');button.form.requestSubmit(button);return true})()`);await until(`document.body.innerText.includes('cancelled')`);
 assert.equal(dialogs,0);console.log('PASS: real Chrome time entry, availability editor, custom booking intake, rescheduling, cancellation, inert markup and mobile overflow; screenshots /tmp/looski-{time-staff,scheduling-staff,booking-mobile}.png');
}finally{
 ws?.close();chrome.kill();await delay(300);rmSync(profile,{recursive:true,force:true});
 for(const [name,enabled] of Object.entries(state.initial)){if(enabled)continue;const headers={'Authorization':'Bearer '+state.token,'Content-Type':'application/json'};const base=state.url+'/api/v1/workspaces/'+state.workspace;const modules=await(await fetch(base+'/modules',{headers})).json();const m=modules.find(m=>m.key===name);await fetch(base+'/modules/'+name,{method:'PATCH',headers:{...headers,'Idempotency-Key':crypto.randomUUID()},body:JSON.stringify({enabled:false,expected_version:m.version})});}
}
