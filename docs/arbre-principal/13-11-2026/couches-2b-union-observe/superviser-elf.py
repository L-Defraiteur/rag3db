from pathlib import Path
import json,os,runpy,sys,time
proof=Path(sys.argv[1]);helper=sys.argv[2];elf=sys.argv[3]
def publish(name,value):
 temporary=proof/(name+'.pending');destination=proof/name
 with temporary.open('x') as f:
  json.dump(value,f,indent=2);f.flush();os.fsync(f.fileno())
 os.link(temporary,destination);temporary.unlink()
parent=os.getpid();pid=os.fork()
if pid==0:
 sys.argv=[helper,str(proof),'lib-large',elf,'--show-output','--test-threads=8']
 runpy.run_path(helper,run_name='__main__')
 os._exit(92)
stamp=(Path('/proc')/str(pid)/'stat').read_text().rsplit(') ',1)[1].split()[19]
publish('elf-wait-parent.json',dict(parent_pid=parent,pid=pid,starttime=stamp,observed=time.monotonic()))
waited,status=os.waitpid(pid,0)
assert waited==pid and (os.WIFEXITED(status) or os.WIFSIGNALED(status))
code=os.waitstatus_to_exitcode(status)
publish('elf-exit-real.json',dict(parent_pid=parent,pid=pid,starttime=stamp,wait_status=status,exit_code=code,observed=time.monotonic()))
sys.exit(code if code>=0 else 128-code)
