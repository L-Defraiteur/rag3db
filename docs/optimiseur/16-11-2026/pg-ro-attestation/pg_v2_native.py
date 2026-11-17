"""Transport process source-only. No CLI, credential loader or implicit connection.

Must run INSIDE a dedicated supervised group and a separately authorized C scope.
This adapter has not been imported/tested/compiled/executed against PostgreSQL.
"""
import hashlib
import os
from pathlib import Path
import re
import selectors
import subprocess
import time
import pg_v2 as v2


class NativeFramesDriver:
    def __init__(self,client,client_sha256,out,expected_cgroup,scope_gate):
        self.client=Path(client);self.client_sha256=client_sha256;self.out=Path(out)
        self.expected_cgroup=expected_cgroup;self.scope_gate=scope_gate
        self.process=None;self.selector=None;self.streams=[];self.stderr=bytearray();self.deadline=None
    def capture(self,payload,deadline):
        self.deadline=deadline
        v2.require(self.scope_gate() is True,'scope_refused')
        v2.require(os.getpgrp()==os.getpid() and os.getsid(0)==os.getpid(),'dedicated_supervisor_group_required')
        v2.require(self.client.is_absolute() and self.client.resolve()==self.client and self.client.stat().st_uid==os.getuid(),'client_path')
        v2.require(not any(x in ('.env','.vault') for x in self.client.parts),'client_path')
        with self.client.open('rb') as stream:actual=hashlib.file_digest(stream,'sha256').hexdigest()
        v2.require(actual==self.client_sha256,'client_pin')
        data=v2.encode(payload);v2.require(len(data)<=8192,'request_bound')
        env=dict(PATH='/usr/bin:/bin',HOME=str(self.out/'home'),TMPDIR=str(self.out/'tmp'),LANG='C.UTF-8',LC_ALL='C.UTF-8',RUST_BACKTRACE='0')
        # Payload stays in memory/pipe; neither request.json nor argv/env contains it.
        self.process=subprocess.Popen([str(self.client),'--capture-readonly-v2'],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,env=env,start_new_session=False)
        from tool_phase import identity
        observed=identity(self.process.pid)
        v2.require(observed.get('uid')==os.getuid() and observed.get('cgroup')==self.expected_cgroup and observed.get('exe')==str(self.client) and 'start' in observed,'native_identity')
        v2.write_new(self.out/'native-identity.json',observed)
        self.selector=selectors.DefaultSelector();self.streams=[self.process.stdin,self.process.stdout,self.process.stderr]
        for stream in self.streams:os.set_blocking(stream.fileno(),False)
        self.selector.register(self.process.stdin,selectors.EVENT_WRITE,'stdin')
        self.selector.register(self.process.stdout,selectors.EVENT_READ,'stdout');self.selector.register(self.process.stderr,selectors.EVENT_READ,'stderr')
        position=0;total=0;pending=bytearray()
        while self.selector.get_map():
            v2.require(self.scope_gate() is True and time.monotonic()<deadline,'original_deadline')
            for key,_ in self.selector.select(max(0.,min(.05,deadline-time.monotonic()))):
                if key.data=='stdin':
                    try:position+=os.write(key.fd,data[position:])
                    except BrokenPipeError:position=len(data)
                    if position==len(data):self.selector.unregister(key.fileobj);key.fileobj.close()
                    continue
                try:chunk=os.read(key.fd,8192)
                except BlockingIOError:continue
                if not chunk:self.selector.unregister(key.fileobj);continue
                if key.data=='stderr':
                    self.stderr.extend(chunk);v2.require(len(self.stderr)<=4096,'stderr_bound');continue
                total+=len(chunk);v2.require(total<=v2.MAX_FRAMES*v2.MAX_FRAME,'stdout_bound');pending.extend(chunk)
                while b'\n' in pending:
                    line,_,rest=pending.partition(b'\n');pending=bytearray(rest)
                    v2.require(len(line)<=v2.MAX_FRAME,'frame_bound')
                    yield bytes(line)+b'\n'
                v2.require(len(pending)<=v2.MAX_FRAME,'frame_bound')
        v2.require(not pending,'partial_frame')
        # No arbitrary native error text is archived (it may contain credentials).
        if self.stderr:
            v2.require(re.fullmatch(rb'PG_V2_REFUSED:(connect_before_attribution|query|readonly|cancel|driver_join|postcheck|timeout|protocol)\n',self.stderr) is not None,'unrecognized_stderr')
    def finish(self,deadline,*,stop):
        if self.selector is not None:self.selector.close()
        for stream in self.streams:
            try:stream.close()
            except OSError:pass
        if self.process is None:return dict(reaped=False,quiet=True,returncode=None)
        if stop and self.process.poll() is None:
            try:self.process.kill()
            except ProcessLookupError:pass
        try:self.process.wait(timeout=max(.001,deadline-time.monotonic()))
        except subprocess.TimeoutExpired:return dict(reaped=False,quiet=False,returncode=None)
        from tool_qualification import group_members
        members,unknown=group_members(os.getpgrp())
        others=[x for x in members if x['pid']!=os.getpid()]
        quiet=not others and not unknown
        return dict(reaped=True,quiet=quiet,returncode=self.process.returncode)


if __name__=='__main__':
    raise SystemExit('PREPARATION_ONLY: separate source/bundle/OS/credential/GO C binding required')
