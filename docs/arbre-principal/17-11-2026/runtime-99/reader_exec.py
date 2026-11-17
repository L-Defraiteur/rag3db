"""One exec bridge for the first real typed query, never a second invocation."""
import hashlib,json,os,sys,time
from pathlib import Path

def main(config_path):
    config=json.loads(Path(config_path).read_text())
    if time.monotonic()>=config['deadline']:raise ValueError('reader exec deadline')
    argv=config['argv'];binary=Path(argv[0])
    if hashlib.sha256(binary.read_bytes()).hexdigest()!=config['sha256']:raise ValueError('reader ELF changed before exec')
    root=Path(config['proof'])
    # stdout/stderr are the native protocol only; GDB has its own RAW logs.
    out=os.open(root/'native.stdout',os.O_WRONLY|os.O_CREAT|os.O_EXCL,0o600)
    err=os.open(root/'native.stderr',os.O_WRONLY|os.O_CREAT|os.O_EXCL,0o600)
    os.dup2(out,1);os.dup2(err,2);os.close(out);os.close(err)
    os.execvpe(argv[0],argv,os.environ.copy())

if __name__=='__main__':main(sys.argv[1])
