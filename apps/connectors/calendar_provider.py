# SPDX-License-Identifier: AGPL-3.0-only
"""Google implements the provider-neutral scheduling connector contract."""
from datetime import datetime, timedelta, timezone
import hashlib
import json
from urllib.error import HTTPError
from urllib.parse import quote, urlencode
from zoneinfo import ZoneInfo


def instant(value):
    result = datetime.fromisoformat(value.replace('Z', '+00:00'))
    if result.tzinfo is None: raise ValueError('Timezone required')
    return result


def event_id(booking):
    return 'looski' + hashlib.sha256(booking.encode()).hexdigest()


class Calendar:
    def __init__(self, google, store, config):
        self.google, self.store, self.config = google, store, config
        self.calendar = config.get('calendar', 'primary')

    def path(self, calendar=None):
        return 'calendar/v3/calendars/' + quote(calendar or self.calendar, safe='') + '/events'

    def busy(self, value):
        start, end = instant(value['start']), instant(value['end'])
        if not timedelta(0) < end - start <= timedelta(days=7): raise ValueError('Invalid calendar window')
        busy = []
        exclude = event_id(value['exclude']) if value.get('exclude') else None
        for calendar in self.config.get('conflict_calendars', [self.calendar]):
            page = None
            while True:
                q = dict(timeMin=start.isoformat(), timeMax=end.isoformat(), singleEvents='true', maxResults=2500)
                if page: q['pageToken'] = page
                data = self.google.call(self.path(calendar) + '?' + urlencode(q))
                zone = ZoneInfo(data.get('timeZone', 'UTC'))
                for item in data.get('items', []):
                    if item.get('status') == 'cancelled' or item.get('transparency') == 'transparent': continue
                    if calendar == self.calendar and item['id'] == exclude: continue
                    def boundary(part):
                        return (instant(part['dateTime']) if 'dateTime' in part
                                else datetime.fromisoformat(part['date']).replace(tzinfo=zone))
                    busy.append(dict(start=boundary(item['start']).isoformat(), end=boundary(item['end']).isoformat()))
                page = data.get('nextPageToken')
                if not page: break
        return {'busy': busy}

    def apply(self, value):
        booking, version = value['id'], value['version']
        if not isinstance(version, int) or version < 1: raise ValueError('Invalid version')
        id = event_id(booking)
        path = self.path() + '/' + id
        digest = hashlib.sha256(json.dumps(value, sort_keys=True).encode()).hexdigest()
        saved = self.store.get('calendar:' + id)
        if saved and saved['version'] > version: return {'state': 'superseded'}
        try:
            current = self.google.call(path)
        except HTTPError as error:
            error.close()
            if error.code not in (404, 410): raise
            current = None
        props = (current or {}).get('extendedProperties', {}).get('private', {})
        if value['state'] == 'cancelled' and saved and saved['version'] == version:
            if saved['digest'] != digest: raise ValueError('Changed booking at same version')
            if current and current.get('status') != 'cancelled': raise ValueError('Calendar changed externally')
            return {'state': 'synced'}
        if current and saved and current.get('etag') != saved['etag']:
            # Do not silently overwrite a manual Google Calendar edit.
            if props.get('suite_version') != str(version) or props.get('suite_digest') != digest:
                raise ValueError('Calendar changed externally; reconcile before retrying')
        if value['state'] == 'cancelled':
            if current and current.get('status') != 'cancelled':
                self.google.call(path + '?sendUpdates=none', method='DELETE', headers={'If-Match': current['etag']})
            self.store.set('calendar:' + id, dict(version=version, etag=None, digest=digest))
            return {'state': 'synced'}
        if value['state'] != 'confirmed': raise ValueError('Invalid booking state')
        if saved and saved['version'] == version:
            if saved['digest'] != digest: raise ValueError('Changed booking at same version')
            if not current or current.get('etag') != saved['etag']:
                raise ValueError('Calendar changed externally; reconcile before retrying')
            return {'state': 'synced'}
        if current and current.get('status') == 'cancelled':
            raise ValueError('Calendar event was deleted externally')
        if props.get('suite_version') == str(version) and props.get('suite_digest') == digest:
            result = current  # Recover a completed write after a lost response.
        else:
            if instant(value['start']) >= instant(value['end']): raise ValueError('Invalid event times')
            body = dict(summary=value['name'], start={'dateTime': value['start']}, end={'dateTime': value['end']},
                        description='Booked in Looski Suite. Manage or cancel through Suite.\n' + value['attendee_name'] + ' <' + value['attendee_email'] + '>',
                        extendedProperties={'private': {'suite_booking': booking, 'suite_version': str(version), 'suite_digest': digest}})
            if current:
                result = self.google.call(path + '?sendUpdates=none', body, method='PATCH', headers={'If-Match': current['etag']})
            else:
                result = self.google.call(self.path() + '?sendUpdates=none', dict(body, id=id), method='POST')
        self.store.set('calendar:' + id, dict(version=version, etag=result['etag'], digest=digest))
        return {'state': 'synced'}
