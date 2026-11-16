"""Physical tracked-file evidence, including initialized gitlinks."""
from pathlib import Path
import hashlib, json, os, stat, subprocess

OLD_APP='1bd32dcd9b0cba098f64728971091c5c7b55c19e'
APP='7425375d66a64b0782672225c59516f048682b00'
R=Path.home()/'.cache/rag3weaver-build'
W=Path.home()/'git_workspaces/rag3db-cx-archi-doc-supervisor-r1'
ARCHIVE=Path.home()/'git_workspaces/rag3db-cx-archi-doc-1bd-proof-r1'
TRANSITION=R/'cx-doc-bac-red-transition-r1'

def git(root,*args):
    return subprocess.check_output(['git','-C',str(root),*args],text=True).strip()

def sha(path):
    with path.open('rb') as stream:return hashlib.file_digest(stream,'sha256').hexdigest()

def tracked(root):
    rows=[]
    for entry in subprocess.check_output(['git','-C',str(root),'ls-files','--stage','-z']).split(b'\0'):
        if not entry:continue
        header,name=entry.split(b'\t',1);mode,oid,stage=header.decode().split()
        assert stage=='0';name=os.fsdecode(name);path=root/name
        row=dict(path=name,mode=mode,oid=oid)
        if mode=='160000':
            assert git(path,'rev-parse','HEAD')==oid and not git(path,'status','--porcelain')
            row['tracked']=tracked(path)
        elif mode=='120000':row['link']=os.readlink(path)
        else:
            st=path.lstat();assert stat.S_ISREG(st.st_mode)
            assert bool(st.st_mode&0o111)==(mode=='100755')
            row.update(size=st.st_size,sha256=sha(path))
        rows.append(row)
    return rows

def gitlink_admins(root):
    rows=[]
    for entry in subprocess.check_output(['git','-C',str(root),'ls-files','--stage','-z']).split(b'\0'):
        if not entry:continue
        header,name=entry.split(b'\t',1);mode,oid,stage=header.decode().split()
        if mode!='160000':continue
        path=root/os.fsdecode(name);admin=Path(git(path,'rev-parse','--absolute-git-dir'))
        assert admin.resolve()==admin and not (admin/'HEAD').is_symlink()
        assert (admin/'HEAD').read_text().strip()==oid, 'gitlink archive must be detached'
        rows.append(dict(path=os.fsdecode(name),gitdir=str(admin),head=oid,head_file_sha256=sha(admin/'HEAD')))
    return rows

def physical_archive(root, expected=None):
    expected=OLD_APP if expected is None else expected
    assert git(root,'rev-parse','HEAD')==expected and not git(root,'status','--porcelain')
    admin=Path(git(root,'rev-parse','--absolute-git-dir'))
    assert admin.resolve()==admin and not (admin/'HEAD').is_symlink()
    assert (admin/'HEAD').read_text().strip()==expected, 'archive must have its own detached HEAD'
    return dict(root=str(root),head=expected,gitdir=str(admin),head_file_sha256=sha(admin/'HEAD'),gitlink_admins=gitlink_admins(root),tracked=tracked(root))

def verify_archive(record):
    assert physical_archive(Path(record['root']), record['head'])==record, 'frozen physical source changed'
