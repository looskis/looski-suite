#!/usr/bin/env python3
"""Collect every page of one selection into a private, atomically finalized JSON file.

Uses the supported CLI/HTTP contract, never database credentials. Selection is JSON
on stdin. Fetch/validation failure leaves no completed artifact. Existing files are never replaced.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
from uuid import UUID


def canonical_selection(selection):
    fields={'collection','parent_id','after','limit','archived','ended'}
    if not isinstance(selection,dict) or set(selection)-fields or not isinstance(selection.get('collection'),str) or not selection['collection']:
        raise ValueError('Invalid export selection')
    result={field:selection.get(field) for field in fields}
    result['limit']=25 if result['limit'] is None else result['limit']
    if type(result['limit']) is not int or not 1<=result['limit']<=25:
        raise ValueError('Invalid export page limit')
    for field in ['parent_id','after']:
        if result[field] is not None:
            if not isinstance(result[field],str):raise ValueError('Invalid export record ID')
            result[field]=str(UUID(result[field]))
    for field in ['archived','ended']:
        if result[field] is not None and type(result[field]) is not bool:raise ValueError('Invalid export lifecycle')
    return result


def collect(fetch, selection, destination, max_pages=10000, *, workspace):
    workspace=str(UUID(workspace))
    selection=canonical_selection(selection)
    if selection['after'] is not None:
        raise ValueError('Collection export must start without a cursor')
    if not 1 <= max_pages <= 1000000:
        raise ValueError('Maximum pages must be between 1 and 1000000')
    destination=Path(destination)
    if destination.exists() or destination.is_symlink():
        raise ValueError('Output already exists; choose a new path')
    temporary=None
    try:
        descriptor,temporary=tempfile.mkstemp(prefix='.looski-export-',dir=destination.parent)
        with os.fdopen(descriptor,'w',encoding='utf-8') as output:
            output.write('{"format":"looski-business-package","format_version":1,"pages":[')
            seen=set();identity=None;page_count=0;record_count=0
            while True:
                page=fetch(selection)
                manifest=page['manifest']
                if canonical_selection(manifest.get('selection'))!=selection or manifest.get('consistency')!='live_authorized_pages':
                    raise ValueError('Export response does not match the requested selection')
                if manifest.get('workspace_id')!=workspace or not isinstance(page.get('items'),list) or not all(isinstance(item,dict) for item in page['items']) or len(page['items'])>selection['limit']:
                    raise ValueError('Invalid export response scope or records')
                if not isinstance(manifest.get('application_version'),str) or not manifest['application_version']:
                    raise ValueError('Missing export application version')
                current_selection=selection
                current=(manifest['format'],manifest['format_version'],manifest['application_version'],manifest['workspace_id'])
                if current[:2]!=('looski-business-json',1) or (identity is not None and identity!=current):
                    raise ValueError('Export contract changed during collection')
                identity=current
                if page_count:output.write(',')
                json.dump(page,output,ensure_ascii=False,separators=(',',':'),allow_nan=False)
                page_count+=1;record_count+=len(page['items'])
                selection=page['next']
                if selection is None:break
                selection=canonical_selection(selection)
                if selection['after'] is None or selection['after']==current_selection['after'] or {k:v for k,v in selection.items() if k!='after'}!={k:v for k,v in current_selection.items() if k!='after'}:
                    raise ValueError('Export continuation changed the requested selection')
                marker=json.dumps(selection,sort_keys=True)
                if marker in seen:raise ValueError('Export continuation repeated')
                seen.add(marker)
                if page_count>=max_pages:raise ValueError('Maximum pages reached before collection completed')
            output.write(','.join([']', '"complete":true','"page_count":'+str(page_count),'"record_count":'+str(record_count)])+'}')
            output.flush();os.fsync(output.fileno())
        # link is atomic and refuses an output created by another process while fetching.
        os.link(temporary,destination)
        directory=os.open(destination.parent,os.O_RDONLY)
        try:os.fsync(directory)
        finally:os.close(directory)
        return {'output':str(destination),'pages':page_count,'records':record_count}
    finally:
        if temporary is not None:os.unlink(temporary)


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--api-url',default='http://localhost:58888')
    parser.add_argument('--workspace',required=True)
    parser.add_argument('--token-file',required=True)
    parser.add_argument('--output',required=True,type=Path)
    parser.add_argument('--binary',default=str(Path(__file__).resolve().parents[1]/'target/debug/looski'))
    parser.add_argument('--max-pages',default=10000,type=int)
    args=parser.parse_args()
    def fetch(selection):
        command=[args.binary,'export','--api-url',args.api_url,'--workspace',args.workspace,'--token-file',args.token_file,'page']
        return json.loads(subprocess.check_output(command,input=json.dumps(selection),text=True))
    try:result=collect(fetch,json.load(sys.stdin),args.output,args.max_pages,workspace=args.workspace)
    except (ValueError,KeyError,OSError,subprocess.CalledProcessError) as failure:
        # CLI errors already use structured, redacted stderr. Do not print response records.
        if isinstance(failure,subprocess.CalledProcessError):raise SystemExit('Export failed; no completed package was published')
        raise SystemExit(str(failure))
    print(json.dumps(result))

if __name__=='__main__':main()
