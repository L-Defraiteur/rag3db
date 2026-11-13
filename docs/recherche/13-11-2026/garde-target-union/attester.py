from pathlib import Path
import hashlib,json,mmap,re,subprocess
def sha(p):
 with Path(p).open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
def git(root,*a):return subprocess.check_output(['git','-C',str(root),*a])
def save(P,n,v):(P/n).write_text(json.dumps(v,indent=2)+'\n')
def attest(C,APP,binary,artifact,builds,P,label):
 W=C.parent.parent
 paths=git(W,'ls-tree','-rz',APP,'--','extension/rag3weaver/templates').split(b'\0');expected={}
 for row in paths:
  if not row:continue
  meta,path=row.split(b'\t');mode,kind,oid=meta.split();assert kind==b'blob' and mode in (b'100644',b'100755')
  rel=path.decode().removeprefix('extension/rag3weaver/')
  data=git(W,'cat-file','blob',oid.decode());assert (C/rel).read_bytes()==data;expected[rel]=data
 assert len(expected)==223
 out=Path(builds[artifact['package_id']])/'builtin_templates.rs';generated=out.read_text()
 # Parse only the two JSON-compatible quoted strings from Rust debug output.
 references={}
 for line in generated.splitlines():
  m=re.match(r'^    \(("(?:[^"\\]|\\.)*"), include_bytes!\(("(?:[^"\\]|\\.)*")\)\),$',line)
  if m:references['templates/'+json.loads(m[1])]=json.loads(m[2])
 assert set(references)==set(expected)
 assert all(references[k]==str(C/k) for k in expected)
 with binary.open('rb') as f,mmap.mmap(f.fileno(),0,access=mmap.ACCESS_READ) as data:
  assert data.find(APP[:9].encode())>=0
  assert all(data.find(v)>=0 for v in expected.values())
 save(P,label+'-templates-contre-git.json',dict(app=APP,count=len(expected),files={k:hashlib.sha256(v).hexdigest() for k,v in expected.items()},generated=str(out),generated_sha256=sha(out),include_paths=references,elf_sha256=sha(binary)))
