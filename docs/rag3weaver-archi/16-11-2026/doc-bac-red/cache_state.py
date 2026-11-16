"""Exclusive empty cache creation and an immutable inventory of the old state."""
from pathlib import Path
import hashlib, os, stat, subprocess

LIMIT = 32 * 1024**3

def require_size(root):
    size=int(subprocess.check_output(['du','-sx','--block-size=1',str(root)],text=True).split()[0])
    assert size <= LIMIT, 'combined session target exceeds 32 GiB; no automatic cleanup'
    return size

def require_root(root):
    st=root.lstat()
    assert root.resolve()==root and stat.S_ISDIR(st.st_mode) and st.st_uid==os.getuid()
    return st

def create_new(root, cache):
    before=require_root(root)
    assert cache.parent==root and cache.name=='native-doc-ea670-v1'
    assert not cache.exists() and not cache.is_symlink(), 'initial cache already exists'
    os.mkdir(cache,mode=0o700)  # Exclusive: an existing name, including a symlink, is rejected.
    after=require_root(root)
    assert (before.st_dev,before.st_ino)==(after.st_dev,after.st_ino)
    st=cache.lstat()
    assert stat.S_ISDIR(st.st_mode) and st.st_uid==os.getuid() and cache.resolve()==cache and st.st_dev==before.st_dev
    assert not list(cache.iterdir()), 'new cache unexpectedly populated'
    return dict(device=st.st_dev,inode=st.st_ino,uid=st.st_uid,path=str(cache))

def require_identity(cache, identity):
    st=cache.lstat()
    assert cache.resolve()==cache and stat.S_ISDIR(st.st_mode)
    assert dict(device=st.st_dev,inode=st.st_ino,uid=st.st_uid,path=str(cache))==identity

def inventory(root, cache):
    require_root(root); rows=[]
    def fail(error):raise error
    for current,dirs,files in os.walk(root,followlinks=False,onerror=fail):
        current=Path(current)
        if current==root:dirs[:]=[name for name in dirs if root/name!=cache]
        for name in sorted(dirs+files):
            path=current/name
            if path==cache:continue
            st=path.lstat();row=dict(path=str(path.relative_to(root)),mode=st.st_mode,
                device=st.st_dev,inode=st.st_ino,uid=st.st_uid)
            if stat.S_ISLNK(st.st_mode):row['link']=os.readlink(path)
            elif stat.S_ISREG(st.st_mode):
                with path.open('rb') as stream:row['sha256']=hashlib.file_digest(stream,'sha256').hexdigest()
                row['size']=st.st_size
            else:assert stat.S_ISDIR(st.st_mode), 'unexpected special object in frozen cache'
            rows.append(row)
    return sorted(rows,key=lambda row:row['path'])

def verify_frozen(root, cache, expected):
    actual=inventory(root,cache)
    assert actual==expected, 'old cache changed; stop without cleanup'
