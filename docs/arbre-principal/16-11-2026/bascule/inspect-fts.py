#!/usr/bin/env python3
"""Read existing filesystem snapshots only; never opens a Lucivy writer."""
from pathlib import Path
import hashlib,json,re,stat,sys

def file_inventory(root):
 if not root.is_dir() or root.is_symlink():raise ValueError('missing or symlink root')
 result=[]
 for path in sorted(root.rglob('*')):
  info=path.lstat()
  if stat.S_ISLNK(info.st_mode):raise ValueError('symlink refused: '+str(path))
  if stat.S_ISDIR(info.st_mode):continue
  if not stat.S_ISREG(info.st_mode):raise ValueError('non-regular file refused')
  digest=hashlib.sha256()
  with path.open('rb') as stream:
   for chunk in iter(lambda:stream.read(1048576),b''):digest.update(chunk)
  result.append(dict(path=path.relative_to(root).as_posix(),size=info.st_size,mode=stat.S_IMODE(info.st_mode),sha256=digest.hexdigest()))
 return result

def inspect(root):
 before=file_inventory(root)
 indexes=[]
 for directory in sorted(root.iterdir()):
  if not directory.is_dir():raise ValueError('unexpected file at FTS root')
  config_path=directory/'_shard_config.json'
  if config_path.stat().st_size>1048576:raise ValueError('oversized config')
  config=json.loads(config_path.read_bytes())
  if not isinstance(config,dict) or not isinstance(config.get('fields'),list):raise ValueError('invalid config structure')
  count=config.get('shards')
  if count is None:count=1
  if type(count) is not int or not 1<=count<=4096:raise ValueError('invalid shard count')
  generation_path=directory/'_generation'
  generation=None
  if generation_path.exists():
   if generation_path.stat().st_size>32:raise ValueError('oversized generation')
   value=generation_path.read_text().strip()
   if not re.fullmatch(r'[0-9]+',value):raise ValueError('invalid generation')
   generation=int(value)
  shards=[]
  for i in range(count):
   shard=directory/f'shard_{i}'
   if not shard.is_dir():raise ValueError('missing shard '+str(shard))
   names=sorted(p.name for p in shard.iterdir())
   if not names:raise ValueError('empty shard '+str(shard))
   if not (shard/'meta.json').is_file():raise ValueError('missing shard meta')
   shards.append(dict(path=shard.name,files=names))
  actual={p.name for p in directory.iterdir() if p.is_dir()}
  if actual!={s['path'] for s in shards}:raise ValueError('unexpected shard directories')
  indexes.append(dict(index_key=directory.name,config=config,generation_file=generation,shards=shards))
 if not indexes:raise ValueError('no indexes')
 after=file_inventory(root)
 if before!=after:raise ValueError('input changed during inspection')
 return dict(qualification='FILESYSTEM_INVENTORY_ONLY',lucivy_open_qualified=False,indexes=indexes,files=before)

if __name__=='__main__':
 try:print(json.dumps(inspect(Path(sys.argv[1])),ensure_ascii=False,indent=2))
 except Exception as error:print(type(error).__name__+': '+str(error),file=sys.stderr);sys.exit(1)
