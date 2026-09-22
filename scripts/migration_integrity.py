"""Validate immutable applied migrations against an installation checksum ledger."""
import hashlib
import os
from urllib.parse import urlsplit,parse_qs,unquote
import json
import re
import subprocess
from pathlib import Path


def source_checksums(directory):
    result={}
    for path in sorted(Path(directory).glob('*.sql')):
        match=re.fullmatch(r'(\d+)_.*\.sql',path.name)
        if not match or match.group(1) in result or path.is_symlink():
            raise ValueError('Invalid or duplicate migration version')
        result[match.group(1)]=hashlib.sha256(path.read_bytes()).hexdigest()
    if not result:raise ValueError('No migration source found')
    return result


def validate(applied,recorded,source,baseline):
    for version in applied:
        expected=recorded.get(version,baseline.get(version))
        if expected is None:raise ValueError('Applied migration has no trusted checksum: '+version)
        if source.get(version)!=expected:raise ValueError('Applied migration was removed or changed: '+version)


def database(sql):
    # Password is supplied only to the child environment, never in arguments
    # or diagnostic output. This runner targets the private Compose database.
    url=urlsplit(Path('/run/secrets/migration_url').read_text().strip())
    if url.scheme not in ('postgres','postgresql') or url.hostname!='db' or url.path!='/postgres':
        raise ValueError('Migration runner requires its private installation database')
    environment={**os.environ,'PGHOST':url.hostname,'PGPORT':str(url.port or 5432),
                 'PGUSER':unquote(url.username or ''),'PGPASSWORD':unquote(url.password or ''),
                 'PGDATABASE':'postgres','PGSSLMODE':parse_qs(url.query).get('sslmode',['disable'])[0]}
    result=subprocess.run(['psql','-X','-qAt','-v','ON_ERROR_STOP=1'],input=sql,text=True,env=environment,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
    if result.returncode:raise RuntimeError('Migration integrity database check failed')
    return result.stdout.strip()


def snapshot():
    applied=json.loads(database("SELECT CASE WHEN to_regclass('supabase_migrations.schema_migrations') IS NULL THEN '[]' ELSE 'null' END"))
    if applied is None:applied=json.loads(database("SELECT coalesce(json_agg(version ORDER BY version),'[]') FROM supabase_migrations.schema_migrations"))
    recorded={}
    if database("SELECT to_regclass('supabase_migrations.looski_checksums') IS NOT NULL")=='t':
        recorded=json.loads(database("SELECT coalesce(json_object_agg(version,sha256),'{}') FROM supabase_migrations.looski_checksums"))
    return applied,recorded


def main():
    import argparse
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('command',choices=['verify','prepare','record']);args=parser.parse_args()
    source=source_checksums('/work/supabase/migrations')
    baseline=json.loads(Path('/work/migration-baseline.json').read_text())
    applied,recorded=snapshot()
    if args.command=='verify':validate(applied,recorded,source,baseline)
    elif args.command=='prepare':
        validate(applied,recorded,source,baseline)
        # Persist attempted source before the CLI can commit any migration.
        # Pending (unapplied) versions may be corrected on a later attempt.
        values=','.join("('"+version+"','"+checksum+"')" for version,checksum in source.items())
        database("BEGIN; CREATE SCHEMA IF NOT EXISTS supabase_migrations; CREATE TABLE IF NOT EXISTS supabase_migrations.looski_checksums(version text PRIMARY KEY,sha256 text NOT NULL CHECK(sha256 ~ '^[a-f0-9]{64}$')); REVOKE ALL ON supabase_migrations.looski_checksums FROM PUBLIC; INSERT INTO supabase_migrations.looski_checksums VALUES "+values+" ON CONFLICT(version) DO UPDATE SET sha256=EXCLUDED.sha256; COMMIT;")
    else:
        if set(applied)!=set(source):raise ValueError('Applied migration inventory does not match release source')
        # Only the lock-holding runner invokes record after the CLI succeeds.
        values=','.join("('"+version+"','"+checksum+"')" for version,checksum in source.items())
        database("BEGIN; CREATE TABLE IF NOT EXISTS supabase_migrations.looski_checksums(version text PRIMARY KEY,sha256 text NOT NULL CHECK(sha256 ~ '^[a-f0-9]{64}$')); REVOKE ALL ON supabase_migrations.looski_checksums FROM PUBLIC; INSERT INTO supabase_migrations.looski_checksums VALUES "+values+" ON CONFLICT(version) DO NOTHING; COMMIT;")
        validate(*snapshot(),source,{})
    print('Migration integrity '+args.command+' passed')

if __name__=='__main__':
    try:main()
    except (ValueError,RuntimeError,OSError) as error:raise SystemExit(str(error))
