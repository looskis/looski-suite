"""Shared isolated-installation transport for behavioral integration suites."""
import http.cookiejar
import json
import os
import subprocess
from urllib.error import HTTPError
from urllib.parse import urlencode
from urllib.request import Request, urlopen, build_opener, HTTPCookieProcessor
from dev import ROOT, configuration

class Fixture:
    def __init__(self):
        self.config=configuration()
        assert self.config['COMPOSE_PROJECT_NAME'].endswith('-identity-check'), 'Dedicated fixture stack required'
        self.environment={**os.environ,**self.config}
        self.base=self.config['LOOSKI_PUBLIC_URL']
        self.directory=ROOT/'.local'/self.config['COMPOSE_PROJECT_NAME']
        owner=json.loads((self.directory/'identity-fixture.json').read_text())
        jar=http.cookiejar.CookieJar()
        self.browser=build_opener(HTTPCookieProcessor(jar))
        with self.browser.open(Request(self.base+'/login',data=urlencode({'email':owner['email'],'password':owner['password']}).encode(),headers={'Origin':self.base,'Accept':'text/html'}),timeout=20) as response:
            assert response.status==200
        self.token=next(cookie.value for cookie in jar if cookie.name=='looski_access')
        self.workspace=self.call('/api/v1/workspaces')[0]['id']

    def sql(self,statement):
        return subprocess.check_output(['docker','compose','exec','-T','db','psql','-U','postgres','-d','postgres','-At','-v','ON_ERROR_STOP=1'],input=statement,text=True,cwd=ROOT,env=self.environment).strip()

    def call(self,path,method='GET',body=None,expected=200,key=None,authenticated=True,headers=None):
        headers={'Content-Type':'application/json',**(headers or {})}
        if authenticated: headers['Authorization']='Bearer '+self.token
        if key: headers['Idempotency-Key']=key
        request=Request(self.base+path,method=method,headers=headers,data=None if body is None else json.dumps(body).encode())
        try:response=urlopen(request,timeout=20)
        except HTTPError as failure:response=failure
        with response:
            payload=json.loads(response.read())
            self.request_id=response.headers.get('X-Request-Id')
            assert response.status==expected,(method,path,response.status,expected,payload)
            return payload

    def cli(self,arguments,body=None):
        import tempfile
        with tempfile.TemporaryDirectory(dir=self.directory) as directory:
            from pathlib import Path
            token_file=Path(directory)/'access-token'
            token_file.write_text(self.token);token_file.chmod(0o600)
            command=[str(ROOT/'target/debug/looski'),arguments[0],'--api-url',self.base,'--token-file',str(token_file),'--workspace',self.workspace,*arguments[1:]]
            return json.loads(subprocess.check_output(command,input=None if body is None else json.dumps(body),text=True))

    def mcp(self,name,arguments):
        import tempfile
        import selectors
        from pathlib import Path
        with tempfile.TemporaryDirectory(dir=self.directory) as directory:
            token_file=Path(directory)/'access-token'
            token_file.write_text(self.token);token_file.chmod(0o600)
            process=subprocess.Popen([str(ROOT/'target/debug/looski'),'mcp','--api-url',self.base,'--token-file',str(token_file)],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.DEVNULL,text=True,bufsize=1)
            selector=selectors.DefaultSelector();selector.register(process.stdout,selectors.EVENT_READ)
            def rpc(method,params=None,identifier=None):
                message={'jsonrpc':'2.0','method':method}
                if params is not None:message['params']=params
                if identifier is not None:message['id']=identifier
                process.stdin.write(json.dumps(message)+'\n');process.stdin.flush()
                if identifier is None:return
                while True:
                    assert selector.select(15),'MCP timed out'
                    reply=json.loads(process.stdout.readline())
                    if reply.get('id')==identifier:
                        assert 'error' not in reply,reply
                        return reply['result']
            try:
                rpc('initialize',{'protocolVersion':'2025-11-25','capabilities':{},'clientInfo':{'name':'fixture','version':'1'}},1)
                rpc('notifications/initialized')
                return rpc('tools/call',{'name':name,'arguments':arguments},2)
            finally:
                selector.close();process.stdin.close()
                try:process.wait(timeout=5)
                except subprocess.TimeoutExpired:process.terminate();process.wait(timeout=5)
