#!/usr/bin/env python3
"""Independent AF_UNIX boundary observation; never imports or runs Rust tests."""
from pathlib import Path
import argparse,ctypes,hashlib,json,os,secrets,socket,stat,struct,subprocess

def probe(root):
    root=Path(root);require=root.resolve()==root and root.is_dir()
    if not require:raise RuntimeError('canonical root required')
    fs=subprocess.check_output(['findmnt','-n','-o','FSTYPE','-T',str(root)],text=True).strip()
    if fs in ['tmpfs','ramfs'] or not fs:raise RuntimeError('disk root required')
    directory=root/('m-p'+secrets.token_hex(4));os.mkdir(directory,0o700)
    st=directory.lstat();rows=[]
    try:
        for size,unicode in [(80,False),(107,False),(108,False),(115,False),(107,True),(108,True)]:
            count=size-len(os.fsencode(directory))-1
            if count<1:raise RuntimeError('probe root too long')
            name=('é'*(count//2)+'a'*(count%2)) if unicode else 'a'*count
            path=directory/name;raw=os.fsencode(path);assert len(raw)==size
            row=dict(path=str(path),bytes=len(raw),characters=len(str(path)),unicode=unicode)
            with socket.socket(socket.AF_UNIX,socket.SOCK_STREAM) as listener:
                try:listener.bind(str(path));row.update(python_bound=True,python_errno=None)
                except OSError as error:row.update(python_bound=False,python_errno=error.errno,python_error=str(error),python_error_type=type(error).__name__)
            if path.exists():path.unlink()
            # Raw Linux syscall with a TERMINATING NUL, matching filesystem-path intent.
            libc=ctypes.CDLL(None,use_errno=True);libc.bind.argtypes=[ctypes.c_int,ctypes.c_void_p,ctypes.c_uint];libc.bind.restype=ctypes.c_int
            address=struct.pack('H',socket.AF_UNIX)+raw+b'\0';buffer=ctypes.create_string_buffer(address,len(address))
            with socket.socket(socket.AF_UNIX,socket.SOCK_STREAM) as listener:
                ctypes.set_errno(0);rc=libc.bind(listener.fileno(),buffer,len(address));number=ctypes.get_errno()
                row.update(raw_nul_bound=rc==0,raw_nul_errno=number if rc else None,raw_addrlen=len(address))
            if path.exists():path.unlink()
            rows.append(row)
        assert all(row['python_bound']==(row['bytes']<=107) and row['raw_nul_bound']==(row['bytes']<=107) for row in rows)
        assert not list(directory.iterdir())
        return dict(root=str(root),filesystem=fs,directory=str(directory),uid=st.st_uid,mode=oct(stat.S_IMODE(st.st_mode)),device=st.st_dev,inode=st.st_ino,rows=rows,meaning='independent Python AF_UNIX and raw Linux bind with terminal NUL; not the errno of the earlier Rust failure',all_sockets_closed=True)
    finally:directory.rmdir()

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--root',required=True);p.add_argument('--output',required=True);a=p.parse_args();record=probe(a.root)
    record['directory_absent']=not Path(record['directory']).exists()
    with Path(a.output).open('x') as stream:json.dump(record,stream,indent=2);stream.write('\n')
    print(json.dumps(record))
