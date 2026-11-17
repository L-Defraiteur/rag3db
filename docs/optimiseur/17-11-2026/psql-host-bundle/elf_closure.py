"""ELF metadata parser only. No execution and no host symlink resolution."""
import os
from pathlib import Path
import posixpath
import re
import stat
import struct


def require(ok,message):
    if not ok:raise ValueError(message)


def image_file(capture,image_path):
    require(image_path.startswith('/'),'nonabsolute image path')
    current=posixpath.normpath(image_path);chain=[]
    for _ in range(40):
        parts=current.lstrip('/').split('/');prefix='/'
        for index,name in enumerate(parts):
            prefix=posixpath.join(prefix,name);path=capture/prefix.lstrip('/')
            st=path.lstat()
            if stat.S_ISLNK(st.st_mode):
                link=os.readlink(path);target=link if link.startswith('/') else posixpath.join(posixpath.dirname(prefix),link)
                current=posixpath.normpath(posixpath.join(target,*parts[index+1:]));chain.append(dict(path=prefix,target=link));break
        else:
            require(stat.S_ISREG(st.st_mode),'image object is not regular')
            require(path.resolve().is_relative_to(capture.resolve()),'image path escapes capture')
            return path,chain
    raise ValueError('image symlink cycle/bound')


def metadata(path):
    require(path.stat().st_size<=64*1024*1024,'ELF exceeds bound')
    raw=path.read_bytes();require(len(raw)>=64,'short ELF')
    h=struct.unpack_from('<16sHHIQQQIHHHHHH',raw)
    require(h[0][:7]==b'\x7fELF\x02\x01\x01' and h[1] in (2,3) and h[2]==62,'unsupported ELF ABI')
    require(h[9]==56 and 0<h[10]<=512 and h[5]+h[9]*h[10]<=len(raw),'invalid program headers')
    headers=[struct.unpack_from('<IIQQQQQQ',raw,h[5]+i*h[9]) for i in range(h[10])]
    for p in headers:require(p[2]+p[5]<=len(raw),'program header outside file')
    interp=[p for p in headers if p[0]==3];require(len(interp)<=1,'multiple interpreters')
    interpreter=raw[interp[0][2]:interp[0][2]+interp[0][5]].rstrip(b'\0').decode('ascii') if interp else None
    dynamic=[p for p in headers if p[0]==2];require(len(dynamic)<=1,'multiple dynamic tables')
    tags={}
    if dynamic:
        p=dynamic[0];require(p[5]%16==0,'invalid dynamic size')
        for offset in range(p[2],p[2]+p[5],16):
            tag,value=struct.unpack_from('<qQ',raw,offset)
            if tag==0:break
            tags.setdefault(tag,[]).append(value)
    needed=[];rpaths=[];soname=None
    if 5 in tags:
        require(len(tags[5])==len(tags.get(10,[]))==1,'invalid string table')
        address=tags[5][0];size=tags[10][0]
        loads=[p for p in headers if p[0]==1 and p[3]<=address and address+size<=p[3]+p[5]]
        require(len(loads)==1 and size<=16*1024*1024,'unmapped string table')
        offset=loads[0][2]+address-loads[0][3];strings=raw[offset:offset+size]
        def text(index):
            require(index<len(strings),'string offset outside table');end=strings.find(b'\0',index);require(end>=0,'unterminated string')
            return strings[index:end].decode('ascii')
        needed=[text(v) for v in tags.get(1,[])]
        rpaths=[text(v) for tag in (15,29) for v in tags.get(tag,[])]
        if 14 in tags:require(len(tags[14])==1,'ambiguous SONAME');soname=text(tags[14][0])
    require(all(re.fullmatch('[A-Za-z0-9_.+~-]+',n) for n in needed),'unsafe DT_NEEDED')
    return dict(elf_class=64,endianness='little',machine=62,type=h[1],interpreter=interpreter,needed=needed,rpaths=rpaths,soname=soname)
