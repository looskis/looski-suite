# SPDX-License-Identifier: AGPL-3.0-only
"""Provider implementations behind the replayable HTTP v1 connector."""
import base64
from datetime import datetime, timezone
from email import policy
from email.message import EmailMessage
from email.parser import BytesParser
from email.utils import getaddresses, formatdate
import hashlib
import imaplib
import json
import re
import smtplib
import ssl
import time
from urllib.error import HTTPError
from urllib.parse import urlencode, quote
from urllib.request import Request, urlopen


def b64(data):
    return base64.urlsafe_b64encode(data).decode().rstrip('=')


def unb64(value):
    return base64.urlsafe_b64decode(value + '=' * (-len(value) % 4))


class Google:
    def __init__(self, credentials):
        self.credentials = credentials
        self.token, self.expires = '', 0

    def call(self, path, body=None, method=None, headers=None):
        if time.time() >= self.expires:
            data = urlencode(dict(client_id=self.credentials['client_id'], client_secret=self.credentials['client_secret'],
                                  refresh_token=self.credentials['refresh_token'], grant_type='refresh_token')).encode()
            with urlopen(Request('https://oauth2.googleapis.com/token', data=data), timeout=15) as response:
                token = json.load(response)
            self.token, self.expires = token['access_token'], time.time() + token['expires_in'] - 60
        req = Request('https://www.googleapis.com/' + path, method=method,
                      data=None if body is None else json.dumps(body).encode(),
                      headers={'Authorization': 'Bearer ' + self.token, 'Content-Type': 'application/json', **(headers or {})})
        with urlopen(req, timeout=20) as response:
            raw = response.read(25_000_001)
            if len(raw) > 25_000_000:
                raise ValueError('Provider response too large')
            return json.loads(raw) if raw else {}


def parse_message(raw, address, source, thread_id=None):
    message = BytesParser(policy=policy.default).parsebytes(raw)
    senders = getaddresses(message.get_all('From', []))
    if len(senders) != 1 or not senders[0][1]:
        return None
    sender = senders[0][1].lower()
    if sender == address.lower():
        return None
    refs = re.findall(r'<[^<>\s]{1,500}>', str(message.get('References', '')))[-30:]
    parent = str(message.get('Message-ID', ''))
    thread_id = thread_id or (refs[0] if refs else str(message.get('In-Reply-To', '')) or parent or source)
    part = message.get_body(preferencelist=('plain',))
    body = part.get_content() if part else '[HTML-only email; view the original in your mail provider.]'
    subject = str(message.get('Subject', ''))[:500]
    event = dict(kind='message.received', message_id=source, thread_id=thread_id, channel='email',
                 sender=sender, destination=address, body=str(body)[:20000], subject=subject,
                 state='received', metadata={'message_id_header': parent,
                 'references': refs, 'has_attachments': any(message.iter_attachments()),
                 'thread_kind': 'direct', 'membership_complete': False})
    # Reply only to the sender. Never trust arbitrary Reply-To or introduce recipients.
    thread = dict(recipient=sender, subject=subject, parent=parent, references=refs, provider_thread=thread_id)
    return event, thread


def reply(value, thread, address):
    message = EmailMessage()
    message['From'], message['To'] = address, thread['recipient']
    message['Subject'] = thread['subject']
    message['Date'] = formatdate(localtime=False)
    message['Message-ID'] = '<looski.' + hashlib.sha256(value['client_ref'].encode()).hexdigest() + '@' + address.split('@')[1] + '>'
    if thread['parent']:
        message['In-Reply-To'] = thread['parent']
        message['References'] = ' '.join((thread['references'] + [thread['parent']])[-30:])
    message.set_content(value['body'])
    return message


class Gmail:
    def __init__(self, config, store, google):
        self.config, self.store, self.google = config, store, google
        self.address = config['address']
        self.verified = False

    def verify(self):
        profile = self.google.call('gmail/v1/users/me/profile')
        if profile['emailAddress'].lower() != self.address.lower():
            raise ValueError('Authorized Google mailbox does not match configured address')
        self.verified = True
        return profile

    def receive(self, id):
        try:
            item = self.google.call('gmail/v1/users/me/messages/' + quote(id, safe='') + '?format=raw')
        except HTTPError as error:
            error.close()
            if error.code == 404:
                return
            raise
        if any(label in item.get('labelIds', []) for label in ('DRAFT', 'SPAM', 'TRASH', 'SENT')):
            return
        parsed = parse_message(unb64(item['raw']), self.address, id, item['threadId'])
        if parsed:
            self.store.incoming('gmail:' + id, *parsed)

    def poll(self):
        profile = self.verify()
        cursor = self.store.get('gmail_history')
        if cursor:
            page = None
            try:
                while True:
                    q = {'startHistoryId': cursor, 'historyTypes': 'messageAdded', 'maxResults': 100}
                    if page: q['pageToken'] = page
                    data = self.google.call('gmail/v1/users/me/history?' + urlencode(q))
                    for history in data.get('history', []):
                        for added in history.get('messagesAdded', []): self.receive(added['message']['id'])
                    page = data.get('nextPageToken')
                    if not page: break
                self.store.set('gmail_history', data.get('historyId', profile['historyId']))
                return
            except HTTPError as error:
                error.close()
                if error.code != 404: raise
        # Bounded initial import; capture history BEFORE paging so arrivals cannot be lost.
        page = None
        while True:
            q = {'maxResults': 100, 'q': 'after:' + self.config['import_after']}
            if page: q['pageToken'] = page
            data = self.google.call('gmail/v1/users/me/messages?' + urlencode(q))
            for item in reversed(data.get('messages', [])): self.receive(item['id'])
            page = data.get('nextPageToken')
            if not page: break
        self.store.set('gmail_history', profile['historyId'])

    def send(self, value, thread):
        if not self.verified: self.verify()
        message = reply(value, thread, self.address)
        self.google.call('gmail/v1/users/me/messages/send', {'raw': b64(message.as_bytes()), 'threadId': thread['provider_thread']})


class Imap:
    def __init__(self, config, store):
        self.config, self.store = config, store
        self.address = config['address']

    def poll(self):
        c = self.config
        with imaplib.IMAP4_SSL(c['imap_host'], c.get('imap_port', 993), ssl_context=ssl.create_default_context(), timeout=20) as mail:
            mail.login(c['username'], c['password'])
            status, _ = mail.select(c.get('folder', 'INBOX'), readonly=True)
            if status != 'OK': raise ValueError('Mailbox unavailable')
            validity = mail.response('UIDVALIDITY')[1][0].decode()
            cursor = self.store.get('imap_cursor', {})
            last = cursor.get('uid', 0) if cursor.get('validity') == validity else 0
            status, data = mail.uid('search', None, 'UID', str(last + 1) + ':*')
            if status != 'OK': raise ValueError('Mailbox search failed')
            # IMAP n:* can return the last UID when n is larger than every UID.
            for uid in sorted(int(u) for u in data[0].split() if int(u) > last)[:100]:
                status, parts = mail.uid('fetch', str(uid), '(BODY.PEEK[] RFC822.SIZE)')
                if status != 'OK': raise ValueError('Mailbox fetch failed')
                raw = next((p[1] for p in parts if isinstance(p, tuple)), None)
                if raw is not None:
                    if len(raw) > 25_000_000: raise ValueError('Message too large')
                    source = 'imap:' + validity + ':' + str(uid)
                    parsed = parse_message(raw, self.address, source)
                    if parsed:
                        # UIDVALIDITY reset does not duplicate a message with a stable Message-ID.
                        header = parsed[0]['metadata']['message_id_header']
                        stable = 'rfc822:' + hashlib.sha256(raw).hexdigest() if not header else 'rfc822:' + header
                        self.store.incoming(stable, *parsed)
                self.store.set('imap_cursor', {'validity': validity, 'uid': uid})

    def send(self, value, thread):
        c = self.config
        message = reply(value, thread, self.address)
        # Implicit TLS only in this first adapter. No cleartext/STARTTLS downgrade.
        with smtplib.SMTP_SSL(c['smtp_host'], c.get('smtp_port', 465), context=ssl.create_default_context(), timeout=20) as smtp:
            smtp.login(c.get('smtp_username', c['username']), c.get('smtp_password', c['password']))
            smtp.send_message(message, from_addr=self.address, to_addrs=[thread['recipient']])
