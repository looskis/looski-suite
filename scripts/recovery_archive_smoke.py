#!/usr/bin/env python3
"""Real pinned age: round trip, wrong keys, truncation and tamper rejection."""
import tempfile
from pathlib import Path
from recovery_archive import generate_identity, encrypt, decrypt

def main():
 with tempfile.TemporaryDirectory(prefix='looski-recovery-smoke-') as directory:
  root=Path(directory);key=root/'identity';other=root/'other';public=generate_identity(key);generate_identity(other)
  source=root/'source';source.write_bytes(b'\x00secret recovery fixture\xff'*10000)
  encrypted=encrypt(source,root/'backup.age',public)
  assert b'secret recovery fixture' not in encrypted.read_bytes()
  output=decrypt(encrypted,root/'restored',key);assert output.read_bytes()==source.read_bytes()
  for label,identity,data in [('wrong-key',other,encrypted.read_bytes()),('truncated',key,encrypted.read_bytes()[:-19]),('tampered',key,encrypted.read_bytes()[:-1]+bytes([encrypted.read_bytes()[-1]^1]))]:
   damaged=root/(label+'.age');damaged.write_bytes(data);destination=root/(label+'.output')
   try:decrypt(damaged,destination,identity)
   except RuntimeError:pass
   else:raise AssertionError(label+' was accepted')
   assert not destination.exists()
  assert not list(root.glob('.looski-recovery-*'))
  print('PASS: age encryption roundtrip, wrong-key/truncation/tamper rejection and no unauthenticated published plaintext')
if __name__=='__main__':main()
