#!/usr/bin/env python3
"""CMS through the gateway as a browser without scripting: application selection, type
builder, media upload, draft editing, publishing, the rendered public page with its
embedded form, stored markup staying inert, session separation and revision restore."""
import re
import secrets
import uuid
from urllib.error import HTTPError
from urllib.parse import urlencode
from urllib.request import HTTPCookieProcessor, HTTPRedirectHandler, Request, build_opener
from forms_browser_html import Forms, attribute_values
from integration_fixture import Fixture
f=Fixture();base=f'/api/v1/workspaces/{f.workspace}';web=f.base+f'/workspaces/{f.workspace}/cms'
PNG=b'\x89PNG\r\n\x1a\n'+b'browser-smoke'
def key():return secrets.token_hex(16)
def form_on(html,action):return next(x for x in Forms(html).forms if x['action']==action)['pairs']
def replace(pairs,**changes):
 changes={k.replace('__','.'):v for k,v in changes.items()}
 return [(k,v) for k,v in pairs if k not in changes]+[(k,v) for k,v in changes.items() if v is not None]
class Stay(HTTPRedirectHandler):
 def redirect_request(self,*args,**kwargs):return None
anonymous=build_opener(Stay,HTTPCookieProcessor())
def fetch(opener,url,data=None,origin=None,headers=None):
 request=Request(url,data=None if data is None else urlencode(data).encode(),headers={'Accept':'text/html',**({'Origin':origin} if origin else {}),**(headers or {})})
 try:response=opener.open(request,timeout=20)
 except HTTPError as error:response=error
 with response:return response.status,response.read().decode(errors='replace'),response.headers,response.url
def staff(url,data=None,expected=200):
 status,html,headers,final=fetch(f.browser,url,data,f.base if data is not None else None)
 assert status==expected,(url,status,expected,html[:600])
 return html,final
def client():return {'CF-Connecting-IP':'203.0.113.'+str(secrets.randbelow(250)+1)}
def public(url,data=None,origin=None,headers=None):return fetch(anonymous,url,data,origin,{**client(),**(headers or {})})
def module(name,enabled):
 current=next(m for m in f.call(base+'/modules') if m['key']==name)
 if current['enabled']!=enabled:f.call(base+'/modules/'+name,'PATCH',{'enabled':enabled,'expected_version':current['version']},key=key())
initial={m['key']:m['enabled'] for m in f.call(base+'/modules')}

# Selection, type builder and entry editing with Forms and CRM off.
module('crm',False);module('forms',False);module('cms',False)
html,_=staff(web);assert 'Content is turned off' in html and 'Enable Content' in html
html,_=staff(web+'?/module',form_on(html,'?/module'));assert 'Create an entry' in html
types,_=staff(web+'/types')
assert 'Create a content type' in types
name='Browser pages '+key()
html,url=staff(web+'/types',replace(form_on(types,''),name=name,description='Pages from the browser test'))
type_id=url.rsplit('/',1)[-1];tpath=base+'/cms/types/'+type_id
def builder(html,op,**changes):return staff(url+'?/save',replace(form_on(html,'?/save'),**changes)+[('op',op)])[0]
html=builder(html,'add',new_label='Intro',new_kind='text',new_required='on')
html=builder(html,'add',new_label='Body',new_kind='rich_text')
html=builder(html,'add',new_label='Hero image',new_kind='media')
html=builder(html,'add',new_label='Sign up',new_kind='form')
html=builder(html,'add',new_label='Scratch',new_kind='number')
fields=f.call(tpath)['fields']
assert [x['key'] for x in fields]==['intro','body','hero_image','sign_up','scratch'] and fields[0]['required']
html=builder(html,'save',field__0__label='Introduction',field__1__help='Say something')
fields=f.call(tpath)['fields'];assert fields[0]['label']=='Introduction' and fields[0]['key']=='intro' and fields[1]['help']=='Say something'
html=builder(html,'up:1');assert [x['key'] for x in f.call(tpath)['fields']][:2]==['body','intro']
html=builder(html,'down:0');assert [x['key'] for x in f.call(tpath)['fields']][:2]==['intro','body']
html=builder(html,'remove:4');assert len(f.call(tpath)['fields'])==4
item=f.call(tpath);public_key=item['public_key']
assert f'/c/{public_key}' in html
print('PASS: application selection and field builder without raw JSON, with Forms and CRM off')

# Media upload through the browser form.
def multipart(pairs,filename,content,content_type):
 boundary=uuid.uuid4().hex;body=b''
 for name,value in pairs:
  body+=f'--{boundary}\r\nContent-Disposition: form-data; name="{name}"\r\n\r\n{value}\r\n'.encode()
 body+=f'--{boundary}\r\nContent-Disposition: form-data; name="file"; filename="{filename}"\r\nContent-Type: {content_type}\r\n\r\n'.encode()+content+f'\r\n--{boundary}--\r\n'.encode()
 return body,'multipart/form-data; boundary='+boundary
library,_=staff(web+'/media');assert 'Library' in library
def upload(html,filename,content,content_type,**changes):
 pairs=[(k,v) for k,v in replace(form_on(html,'?/upload'),**changes) if k!='file']
 body,kind=multipart(pairs,filename,content,content_type)
 request=Request(web+'/media?/upload',data=body,headers={'Accept':'text/html','Origin':f.base,'Content-Type':kind})
 try:response=f.browser.open(request,timeout=30)
 except HTTPError as error:response=error
 with response:return response.status,response.read().decode(errors='replace'),response.url
status,rejected,_=upload(library,'page.html',b'<script>alert(1)</script>','text/html',title='Bad');assert status==400 and 'PNG, JPEG' in rejected
status,rejected,_=upload(rejected,'disguised.png',b'<script>alert(1)</script>','image/png',title='Disguised');assert status==400 and 'content matches' in rejected and 'remains private' in rejected
# Choosing a different file after a refusal works from the page that reported it.
status,detail,media_url=upload(rejected,'hero.png',PNG,'image/png',title='Hero picture',alt_text='A picture "of" <heroes>');assert status==200,detail[:400]
media_id=media_url.rsplit('/',1)[-1];assert 'Hero picture' in detail
status,bytes_html,headers,_=fetch(f.browser,media_url+'/content');assert status==200 and headers['X-Content-Type-Options']=='nosniff' and headers['Content-Type']=='image/png'
print('PASS: media upload refuses scriptable files and keeps the sandboxed staff view')

# Draft, publish and the rendered page.
slug='browser-'+secrets.token_hex(3);xss='<script>alert("stored")</script><img src=x onerror=alert(2)>'
home,_=staff(web)
html,entry_url=staff(web+'?/create',replace(form_on(home,'?/create'),type_id=type_id,title='Browser page',slug=slug))
entry_id=entry_url.rsplit('/',1)[-1];epath=base+'/cms/entries/'+entry_id
assert 'Forms is off' in html,'The editor explains a form field while Forms is off'
status,_,_,_=public(f'{f.base}/c/{public_key}/{slug}');assert status==404,'A draft has no public page'
body='# Welcome\n\n'+xss+'\n\nRead **bold** [the docs](https://example.com/docs).\n\n- one\n- two'
def edit(html,**changes):return replace(form_on(html,'?/save'),**changes)
page,_=staff(entry_url+'?/save',edit(html,title=xss,value__intro='Hello '+xss,value__body=body,value__hero_image=media_id))
assert f.call(epath)['draft']['values']['body']==body
rejected,_=staff(entry_url+'?/save',edit(page,value__body='[bad](javascript:alert(1))'),expected=400)
assert 'Links must start with' in rejected and 'javascript:alert(1)' in rejected,'A rejected save keeps what was typed'
assert f.call(epath)['draft']['values']['body']==body
preview,_=staff(entry_url+'/preview');assert 'Draft preview' in preview and 'Welcome' in preview and '<script>alert("stored")' not in preview
assert f'/cms/media/{media_id}/content' in preview
page,_=staff(entry_url)
published,_=staff(entry_url+'?/saveAndPublish',edit(page));assert 'View public page' in published
live=f.call(epath)['published'];assert live['revision_number']==f.call(epath)['draft']['revision_number']
status,rendered,headers,_=public(f'{f.base}/c/{public_key}/{slug}')
assert status==200 and 'Welcome' in rendered and '<strong>bold</strong>' in rendered
assert 'href="https://example.com/docs"' in rendered and '<li>one</li>' in re.sub('<!--.*?-->','',rendered)
# Stored markup is escaped text, in the title element, the heading and the body alike.
assert '<script>alert(' not in rendered and '<img src=x' not in rendered
assert rendered.count('&lt;script') >= 3,'Title, heading and body each show the markup as text'
tags=attribute_values(rendered)
assert not tags['script'],'Published pages ship no scripts'
assert [src for src in tags['img']]==[f'/api/v1/public/cms/{public_key}/media/{media_id}']
policy=headers['Content-Security-Policy']
assert "default-src 'none'" in policy and "frame-ancestors 'none'" in policy and 'script-src' not in policy and "frame-src 'self'" in policy and "form-action 'none'" in policy,policy
assert headers['X-Frame-Options']=='DENY' and headers['Referrer-Policy']=='same-origin' and headers['Cache-Control']=='no-store'
status,_,image,_=public(f'{f.base}/api/v1/public/cms/{public_key}/media/{media_id}');assert status==200 and image['Content-Type']=='image/png'
assert 'not currently available' not in rendered,'No form was chosen yet'
status,index,_,_=public(f'{f.base}/c/{public_key}');assert status==200 and f'/c/{public_key}/{slug}' in index and 'Pages from the browser test' in index
status,missing,headers,_=public(f'{f.base}/c/{public_key}/nothing-here');assert status==404 and "default-src 'none'" in headers['Content-Security-Policy'] and not attribute_values(missing)['script']
status,_,_,_=public(f'{f.base}/c/{"x"*22}');assert status==404
# The renderer never reads a session: a staff cookie changes nothing, the gateway strips it.
token=f.token
status,same,_,_=public(f'{f.base}/c/{public_key}/{slug}',headers={'Cookie':'looski_access='+token})
assert status==200 and 'Draft preview' not in same
print('PASS: rendered page serves published content with inert markup, no scripts, a closed content policy and no session')

# Later edits stay out of the public page; revisions restore through the interface.
page,_=staff(entry_url)
page,_=staff(entry_url+'?/save',edit(page,value__intro='Newer unpublished words',title='Newer title'))
status,rendered,_,_=public(f'{f.base}/c/{public_key}/{slug}');assert 'Newer unpublished words' not in rendered and 'Newer title' not in rendered
history,history_url=staff(entry_url+'/revisions');assert 'Live' in history and 'Restore' in history
rows=f.call(epath+'/revisions')['items'];target=rows[-1]
one,_=staff(history_url+'/'+target['id']);assert f'Revision {target["number"]}' in one
restore=next(x['pairs'] for x in Forms(history).forms if x['action']=='?/restore' and ('revision_id',live['revision_id']) in [tuple(p) for p in x['pairs']])
page,_=staff(history_url+'?/restore',restore)
assert f.call(epath)['draft']['values']['intro']=='Hello '+xss and f.call(epath)['draft']['revision_number']>rows[0]['number']
status,rendered,_,_=public(f'{f.base}/c/{public_key}/{slug}');assert status==200
page,_=staff(entry_url+'?/saveAndPublish',edit(page,value__intro='Restored and published'))
status,rendered,_,_=public(f'{f.base}/c/{public_key}/{slug}');assert 'Restored and published' in rendered
print('PASS: unpublished edits stay private and a revision restores through the web interface')

# The embedded form: unavailable while Forms is off, then submitted inside its frame.
module('forms',True)
# Exercise the picker beyond the API's first page, including on a fresh fixture.
existing=f.call(base+'/forms?limit=100')['items']
for index in range(100-len(existing)):
 f.call(base+'/forms','POST',{'name':'Picker boundary '+key(),'draft':{'fields':[{'key':'name','label':'Name','kind':'short_text'}]}},expected=201,key=key())
form=f.call(base+'/forms','POST',{'name':'Embedded '+key(),'confirmation_message':'Thanks from the frame.','draft':{'fields':[{'key':'name','label':'Your name','kind':'short_text','required':True},{'key':'email','label':'Email','kind':'email'}]}},expected=201,key=key())
form=f.call(base+'/forms/'+form['id']+'/publish','POST',{'expected_version':form['version']},key=key())
assert form['id'] not in [item['id'] for item in f.call(base+'/forms?limit=100')['items']]
page,_=staff(entry_url);assert form['name'] in page
page,_=staff(entry_url+'?/saveAndPublish',edit(page,value__sign_up=form['id']))
status,rendered,_,_=public(f'{f.base}/c/{public_key}/{slug}')
tags=attribute_values(rendered);frame=f'/f/{form["public_key"]}/embed'
assert tags['iframe']==[frame] and not tags['script'] and f'href="/f/{form["public_key"]}"' in rendered
status,embed,headers,_=public(f.base+frame);assert status==200 and 'frame-ancestors *' in headers['Content-Security-Policy']
pairs=replace(next(x['pairs'] for x in Forms(embed).forms),answer__name='Frame Visitor')
status,done,_,_=public(f.base+frame,pairs,f.base);assert status==200 and 'Thanks from the frame.' in done,done[:400]
status,_,_,_=public(f.base+frame,pairs,'https://evil.example');assert status==403
assert f.call(base+'/forms/'+form['id']+'/submissions')['items'][0]['answers']['name']=='Frame Visitor'
module('forms',False)
status,rendered,_,_=public(f'{f.base}/c/{public_key}/{slug}')
assert status==200 and 'This form is not currently available.' in rendered and form['public_key'] not in rendered and not attribute_values(rendered)['iframe']
page,_=staff(entry_url);assert 'current choice is kept' in page
page,_=staff(entry_url+'?/save',edit(page,value__intro='Edited with Forms off'))
assert f.call(epath)['draft']['values']['sign_up']==form['id'],'An existing form choice survives editing while Forms is off'
print('PASS: form beyond the first picker page embeds, accepts answers inside its frame, and degrades with Forms off')

# Session gates and application disablement on the web surface.
status,_,headers,_=public(entry_url);assert status==303 and '/login' in headers['Location']
status,_,headers,_=public(media_url+'/content');assert status in (303,401)
status,_,_,_=fetch(f.browser,web+'?/module',form_on(staff(web)[0],'?/module'),'https://evil.example');assert status==403
module('cms',False)
status,_,_,_=public(f'{f.base}/c/{public_key}/{slug}');assert status==404
html,_=staff(web);assert 'Content is turned off' in html
for k,enabled in initial.items():module(k,enabled)
print('PASS: session gates, forged origins and application disablement close the web surface')
