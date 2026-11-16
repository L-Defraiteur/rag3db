"""Decode bounded native framing; snapshots persist row digests, never corpus values."""
import hashlib,struct,collections
MAX_ROW=16*1024**2

def exact(stream,n):
 data=stream.read(n)
 if len(data)!=n:raise ValueError('truncated typed stream')
 return data

def integer(stream):return struct.unpack('<Q',exact(stream,8))[0]
def binary(stream):
 n=integer(stream)
 if n>MAX_ROW:raise ValueError('typed value exceeds bound')
 return exact(stream,n)

def decode_value(stream,depth=0):
 if depth>64:raise ValueError('typed nesting exceeds bound')
 typename=binary(stream).decode('ascii');null=exact(stream,1)
 if null==b'0':return (typename,None)
 if null!=b'1':raise ValueError('invalid null flag')
 tag=exact(stream,1)
 if tag==b's':v=binary(stream)
 elif tag==b'n':v=binary(stream).decode('ascii')
 elif tag==b'f':v=struct.unpack('<f',exact(stream,4))[0]
 elif tag==b'd':v=struct.unpack('<d',exact(stream,8))[0]
 elif tag==b'a':
  n=integer(stream)
  if n>MAX_ROW:raise ValueError('nested value count exceeds bound')
  v=[decode_value(stream,depth+1) for _ in range(n)]
 else:raise ValueError('invalid typed tag')
 return typename,v

def consume(stream,callback=None):
 import io
 if exact(stream,8)!=b'R3TYP002':raise ValueError('bad typed magic')
 columns=integer(stream);count=integer(stream)
 if columns>4096:raise ValueError('typed column bound')
 hashes=collections.Counter()
 for _ in range(count):
  raw=binary(stream);row=io.BytesIO(raw);values=[decode_value(row) for _ in range(columns)]
  if row.read(1):raise ValueError('typed row has trailing bytes')
  hashes[hashlib.sha256(raw).hexdigest()]+=1
  if callback:callback(values)
 if stream.read(1):raise ValueError('typed stream has trailing bytes')
 return dict(columns=columns,count=count,rows=dict(sorted(hashes.items())))

def text(value):
 if value[0]!='STRING' or not isinstance(value[1],bytes):raise ValueError('expected native STRING')
 return value[1].decode('utf-8')
