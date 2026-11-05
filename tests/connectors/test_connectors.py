import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import Mock, patch
from urllib.error import HTTPError
sys.path.insert(0, str(Path(__file__).resolve().parents[2] / 'apps/connectors'))
from store import Store
from providers import Gmail, Imap, parse_message, reply
from calendar_provider import Calendar, event_id


class Connectors(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.path = str(Path(self.directory.name) / 'journal.sqlite')
        self.store = Store(self.path, 'test')
        self.value = dict(version=1, client_ref='first', thread_id='thread', channel='email', body='Reply', subject='Question')
        self.store.incoming('one', dict(kind='message.received', message_id='one', thread_id='thread'),
                            dict(recipient='person@example.com', subject='Question', parent='<a@example.com>', references=[], provider_thread='thread'))
    def tearDown(self): self.directory.cleanup()
    def test_durable_idempotency_and_changed_replay(self):
        result = self.store.enqueue(self.value)
        self.assertEqual(result, Store(self.path, 'test').enqueue(self.value))
        with self.assertRaises(ValueError): self.store.enqueue(dict(self.value, body='Changed'))
        with self.assertRaises(ValueError): Store(self.path, 'different')
    def test_ambiguous_send_never_retries(self):
        self.store.enqueue(self.value)
        provider = Mock(); provider.send.side_effect = TimeoutError()
        self.store.drain(provider); self.store.drain(provider)
        self.assertEqual(provider.send.call_count, 1)
        self.assertEqual(self.store.events(0, 50)[-1]['state'], 'unknown')
    def test_crash_after_send_claim_is_unknown(self):
        self.store.enqueue(self.value)
        with self.store.connect() as db: db.execute("UPDATE outbound SET state='sending'")
        recovered = Store(self.path, 'test'); provider = Mock(); recovered.drain(provider)
        provider.send.assert_not_called()
        self.assertEqual(recovered.events(0, 50)[-1]['state'], 'unknown')
    def test_received_dedupe_and_cursor(self):
        self.store.incoming('one', dict(thread_id='thread'), {})
        self.assertEqual(len(self.store.events(0,50)), 1)
        self.assertEqual(self.store.events(1,50), [])
    def test_reply_uses_native_headers_and_sender_only(self):
        parsed = parse_message(b'From: Person <person@example.com>\r\nReply-To: attacker@example.com\r\nTo: me@example.com\r\nSubject: Original\r\nMessage-ID: <two@example.com>\r\nReferences: <one@example.com>\r\n\r\nHello', 'me@example.com', '2')
        self.assertEqual(parsed[0]['thread_id'], '<one@example.com>')
        mail = reply(self.value, parsed[1], 'me@example.com')
        self.assertEqual(mail['To'], 'person@example.com')
        self.assertEqual(mail['In-Reply-To'], '<two@example.com>')
        self.assertEqual(mail['Subject'], 'Original')
    def test_gmail_history_failure_preserves_cursor(self):
        self.store.set('gmail_history', '10')
        google = Mock(); google.call.side_effect = [dict(emailAddress='me@example.com', historyId='20'), OSError('offline')]
        with self.assertRaises(OSError): Gmail(dict(address='me@example.com',import_after='2026/10/01'),self.store,google).poll()
        self.assertEqual(self.store.get('gmail_history'), '10')
    def test_gmail_expired_history_full_sync_captures_pre_scan_cursor(self):
        self.store.set('gmail_history', '10')
        google = Mock(); google.call.side_effect = [dict(emailAddress='me@example.com', historyId='20'), HTTPError('',404,'expired',{},None), {'messages':[]}]
        Gmail(dict(address='me@example.com',import_after='2026/10/01'),self.store,google).poll()
        self.assertEqual(self.store.get('gmail_history'), '20')
    def test_wrong_account_never_imports(self):
        google = Mock(); google.call.return_value = dict(emailAddress='other@example.com', historyId='20')
        with self.assertRaises(ValueError): Gmail(dict(address='me@example.com',import_after='2026/10/01'),self.store,google).poll()
        self.assertEqual(google.call.call_count, 1)
    def test_imap_uidvalidity_reset_deduplicates_and_never_marks_read(self):
        mail=Mock(); mail.__enter__=Mock(return_value=mail);mail.__exit__=Mock(return_value=False)
        mail.select.return_value=('OK', [b'1']);mail.response.return_value=('UIDVALIDITY',[b'12'])
        raw=b'From: person@example.com\r\nTo: me@example.com\r\nSubject: Question\r\nMessage-ID: <imap-one@example.com>\r\n\r\nHello'
        def uid(command,*args):
            if command=='search':return ('OK',[b'1'])
            self.assertIn('BODY.PEEK[]',args[1]);return ('OK',[(b'1',raw)])
        mail.uid.side_effect=uid
        config=dict(address='me@example.com',imap_host='example.invalid',username='me',password='private')
        with patch('providers.imaplib.IMAP4_SSL',return_value=mail):
            Imap(config,self.store).poll()
            mail.select.assert_called_with('INBOX',readonly=True)
            self.assertEqual(self.store.get('imap_cursor'),{'validity':'12','uid':1})
            mail.response.return_value=('UIDVALIDITY',[b'13'])
            Imap(config,self.store).poll()
        self.assertEqual(len(self.store.events(1,50)),1)
        self.assertEqual(self.store.get('imap_cursor'),{'validity':'13','uid':1})
    def test_imap_fetch_failure_keeps_last_committed_uid(self):
        mail=Mock(); mail.__enter__=Mock(return_value=mail);mail.__exit__=Mock(return_value=False)
        mail.select.return_value=('OK',[b'1']);mail.response.return_value=('UIDVALIDITY',[b'12'])
        mail.uid.side_effect=[('OK',[b'2']),('NO',[b'failed'])]
        self.store.set('imap_cursor',{'validity':'12','uid':1})
        with patch('providers.imaplib.IMAP4_SSL',return_value=mail), self.assertRaises(ValueError):
            Imap(dict(address='me@example.com',imap_host='example.invalid',username='me',password='private'),self.store).poll()
        self.assertEqual(self.store.get('imap_cursor'),{'validity':'12','uid':1})
    def test_http_connector_authentication_and_durable_enqueue(self):
        import server, threading, time
        from http.server import HTTPServer
        from urllib.request import Request, urlopen
        config=Path(self.directory.name)/'config.json'
        config.write_text(json.dumps(dict(provider='gmail',address='me@example.com',installation_id='test',api_token='x'*40,google={},import_after='2026/10/01')))
        config.chmod(0o600)
        provider=Mock();servers=[]
        def capture(address,handler):
            instance=HTTPServer(('127.0.0.1',0),handler);servers.append(instance);return instance
        with patch.object(sys,'argv',['server.py','--config',str(config)]), patch('server.Gmail',return_value=provider), patch('server.HTTPServer',side_effect=capture):
            thread=threading.Thread(target=server.main,daemon=True);thread.start()
            try:
                for _ in range(100):
                    if servers:break
                    time.sleep(.01)
                self.assertTrue(servers)
                url='http://127.0.0.1:'+str(servers[0].server_port)
                with self.assertRaises(HTTPError) as denied: urlopen(url+'/v1/capabilities')
                self.assertEqual(denied.exception.code,401);denied.exception.close()
                def post(value):
                    req=Request(url+'/v1/messages',data=json.dumps(value).encode(),headers={'Authorization':'Bearer '+'x'*40})
                    with urlopen(req,timeout=2) as response:return json.load(response)
                result=post(self.value)
                self.assertEqual(result,post(self.value))
                with self.assertRaises(HTTPError) as changed: post(dict(self.value,body='Changed'))
                self.assertEqual(changed.exception.code,409);changed.exception.close()
                self.assertEqual(self.store.enqueue(self.value),result)
            finally:
                if servers:servers[0].shutdown()
                thread.join(timeout=3)
    def booking(self, **values):
        return dict(dict(id='abc',version=1,state='confirmed',name='Meeting', start='2026-11-06T10:00:00Z',end='2026-11-06T10:30:00Z',attendee_name='Person',attendee_email='person@example.com'), **values)
    def test_calendar_all_day_dst_and_pagination(self):
        google = Mock(); google.call.side_effect = [dict(timeZone='America/Los_Angeles', items=[dict(id='1',start={'date':'2026-11-01'},end={'date':'2026-11-02'})],nextPageToken='next'),dict(items=[dict(id='2',transparency='transparent')])]
        result = Calendar(google,self.store,{}).busy(dict(start='2026-11-01T00:00:00Z',end='2026-11-03T00:00:00Z'))
        self.assertEqual(result['busy'], [dict(start='2026-11-01T00:00:00-07:00',end='2026-11-02T00:00:00-08:00')])
        self.assertIn('pageToken=next', google.call.call_args[0][0])
    def test_calendar_idempotency_and_external_edit(self):
        google = Mock(); current = None
        def call(path, body=None, method=None, headers=None):
            nonlocal current
            if body:
                current = dict(body, etag='first'); return current
            if current is None: raise HTTPError('',404,'missing',{},None)
            return current
        google.call.side_effect = call
        calendar = Calendar(google,self.store,{})
        calendar.apply(self.booking()); calendar.apply(self.booking())
        self.assertEqual(sum(c.kwargs.get('method')=='POST' for c in google.call.call_args_list),1)
        current['etag']='manual-edit'
        with self.assertRaises(ValueError): calendar.apply(self.booking())
        with self.assertRaises(ValueError): calendar.apply(self.booking(version=2))
    def test_cancel_retry_and_older_version(self):
        google=Mock(); google.call.return_value={'status':'cancelled', 'etag':'deleted'}
        calendar=Calendar(google,self.store,{})
        self.assertEqual(calendar.apply(self.booking(version=2,state='cancelled'))['state'],'synced')
        self.assertEqual(calendar.apply(self.booking(version=2,state='cancelled'))['state'],'synced')
        self.assertEqual(calendar.apply(self.booking())['state'],'superseded')
    def test_free_busy_failure_does_not_report_free(self):
        google=Mock();google.call.side_effect=OSError('offline')
        with self.assertRaises(OSError): Calendar(google,self.store,{}).busy(dict(start='2026-11-01T00:00:00Z',end='2026-11-02T00:00:00Z'))

if __name__=='__main__': unittest.main()
