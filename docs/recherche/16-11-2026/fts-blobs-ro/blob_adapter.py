"""Strict binary transport and materialization; no DB or Lucivy writer API."""
import hashlib,json,re,struct
from pathlib import Path

def decode_frames(raw):
    if len(raw)<16 or raw[:8]!=b'RFTSB001':raise ValueError('invalid binary header')
    count=struct.unpack_from('<Q',raw,8)[0];position=16;rows={}
    if count>(len(raw)-16)//16:raise ValueError('impossible row count')
    for _ in range(count):
        if position+16>len(raw):raise ValueError('truncated lengths')
        key_size,data_size=struct.unpack_from('<QQ',raw,position);position+=16
        if not key_size or key_size+data_size>len(raw)-position:raise ValueError('invalid or truncated row')
        try:key=raw[position:position+key_size].decode('utf-8')
        except UnicodeError as error:raise ValueError('non-UTF8 key') from error
        position+=key_size;data=raw[position:position+data_size];position+=data_size
        if '\x00' in key or key in rows:raise ValueError('null or duplicate key')
        rows[key]=data
    if position!=len(raw):raise ValueError('trailing bytes')
    return rows

def materialize(blobs,index,directory):
    directory=Path(directory)
    flat=lambda name:bool(re.fullmatch(r'[A-Za-z0-9_.-]+',name)) and name not in ['.','..']
    if not flat(index) or not index.startswith('Lucivy_'):raise ValueError('invalid exact index name')
    if directory.exists() or directory.is_symlink() or directory.absolute()!=directory.resolve():raise ValueError('output must be new and canonical')
    root_prefix=index+'/';shard_prefix='Lucivy_'+index+'/'
    roots={key[len(root_prefix):]:data for key,data in blobs.items() if key.startswith(root_prefix)}
    if not roots or any(not flat(name) for name in roots):raise ValueError('invalid root inventory')
    try:config=json.loads(roots['_shard_config.json'])
    except (KeyError,ValueError,UnicodeError) as error:raise ValueError('missing or invalid shard config') from error
    if not isinstance(config,dict):raise ValueError('shard config must be an object')
    count=config.get('shards',1)
    if type(count) is not int or count<=0 or count>len(blobs):raise ValueError('invalid shard count')
    planned=dict(roots);shards={f'shard_{i}':{} for i in range(count)}
    for key,data in blobs.items():
        if not key.startswith(shard_prefix):continue
        parts=key[len(shard_prefix):].split('/')
        if len(parts)!=2 or parts[0] not in shards or not flat(parts[1]):raise ValueError('invalid shard inventory or path')
        shards[parts[0]][parts[1]]=data
    for shard,files in shards.items():
        if 'meta.json' not in files or shard in roots:raise ValueError('missing shard metadata or conflicting path')
        planned.update({shard+'/'+name:data for name,data in files.items()})
    # Every path/config/inventory check precedes the first directory creation.
    directory.mkdir(mode=0o700)
    for shard in shards:(directory/shard).mkdir(mode=0o700)
    manifest={}
    for name,data in sorted(planned.items()):
        with (directory/name).open('xb') as output:output.write(data)
        if (directory/name).read_bytes()!=data:raise ValueError('materialized bytes differ')
        manifest[name]=dict(bytes=len(data),sha256=hashlib.sha256(data).hexdigest())
    return manifest
