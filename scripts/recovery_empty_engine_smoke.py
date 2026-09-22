#!/usr/bin/env python3
"""Verify portable images in an empty, offline disposable Docker-in-Docker engine."""
import argparse,json,secrets,subprocess,tempfile,time
from pathlib import Path
import recovery_archive as encryption
import recovery_bundle as bundle
import recovery_images as image_bundle
parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--archive',required=True,type=Path);parser.add_argument('--identity',required=True,type=Path);args=parser.parse_args()
image='docker:28.5.1-dind@sha256:ea9d20492ca1caaaba78e68453433895d256173c79281756e88b745647fcbcfd'
name='looski-empty-recovery-'+secrets.token_hex(5)
with tempfile.TemporaryDirectory(prefix='looski-empty-recovery-') as directory:
 root=Path(directory);archive=root/'snapshot.tar';encryption.decrypt(args.archive,archive,args.identity)
 manifest=bundle.extract(archive,root/'bundle');images=manifest['metadata']['images']
 # Privilege is confined to this disposable engine container. It has no host
 # filesystem/socket mounts, no network and no published ports. Its anonymous
 # data volume is removed with this container only; no host image is deleted.
 subprocess.run(['docker','run','-d','--name',name,'--privileged','--network','none','-e','DOCKER_TLS_CERTDIR=','--volume','/var/lib/docker',image,'dockerd','--host=unix:///var/run/docker.sock','--storage-driver=vfs','--iptables=false','--bridge=none'],check=True,stdout=subprocess.DEVNULL)
 try:
  deadline=time.monotonic()+60
  while subprocess.run(['docker','exec',name,'docker','info'],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL).returncode:
   if time.monotonic()>deadline:raise AssertionError('Disposable engine did not start')
   time.sleep(.5)
  assert not subprocess.check_output(['docker','exec',name,'docker','image','ls','-q']).strip()
  with (root/'bundle/images.tar.gz').open('rb') as source:
   subprocess.run(['docker','exec','-i',name,'docker','image','load','--quiet'],stdin=source,check=True,stdout=subprocess.DEVNULL,stderr=subprocess.PIPE)
  restored=image_bundle.loaded_ids(images,('docker','exec',name,'docker'))
  assert all(restored[s]==images[s]['config_id'] for s in images)
  print('PASS: '+str(len(set(restored.values())))+' exact service images loaded into an initially empty offline Docker engine without host cache access')
 finally:
  subprocess.run(['docker','rm','-f','-v',name],check=True,stdout=subprocess.DEVNULL)
