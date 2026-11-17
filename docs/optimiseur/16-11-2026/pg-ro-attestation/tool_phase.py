"""Phase native interne future. Jamais appelée en PREPA ou par les faux CPU.
Le pilote extérieur borne tout le groupe OS, y compris ce code de capture.
"""
import json
import os
from pathlib import Path
import selectors
import subprocess
import sys
import time
import tool_qualification as tool


def identity(pid):
    root=Path('/proc')/str(pid)
    try:
        fields=(root/'stat').read_text().rsplit(')',1)[1].split()
        return {'pid':pid,'start':int(fields[19]),'uid':root.stat().st_uid,
                'cgroup':(root/'cgroup').read_text().strip(),'exe':os.readlink(root/'exe')}
    except (FileNotFoundError,ProcessLookupError):return {'pid':pid,'exited_before_snapshot':True}


def main():
    request_path=Path(sys.argv[1]).absolute();body=tool.read_json(request_path,sys.argv[2])
    root=Path(__file__).resolve().parent;packet=tool.read_json(root/'packet.json',body['packet_sha'])
    tool.common_authority(root,packet);tool.verify_sources(root,packet)
    tool.validate_go(body['go'],packet,body['packet_sha'],time.monotonic())
    actual_parent=identity(os.getppid())
    for key in ['pid','start','uid','cgroup']:
        tool.require(actual_parent.get(key)==body['parent'].get(key),'phase parent identity differs')
    phase=Path(body['phase']);out=Path(body['go']['output'])
    tool.require(phase.parent==out and request_path.parent==phase,'phase paths differ')
    req=body['request'];deadline=body['deadline'];started=body['started']
    if req['kind']=='compile':expected=tool.compile_argv(packet,out)
    else:
        cases=tool.read_json(root/'tool-qualification-cases.json')['cases']
        case=next((x for x in cases if x['name']==req['name']),None)
        tool.require(case is not None,'unknown parser fixture')
        expected=[str(out/'client'),*case['argv']]
        tool.require(case['stdin']==req['stdin'] and case['argv'] in ([],['--validate-request']),'fixture or mode differs')
    tool.require(req['argv']==expected and time.monotonic()<deadline-.4,'argv or original budget refused')
    native=None;fds=[];reason=None;overflow=False;reaped=False;stdin_started=None
    buffers={'stdout':bytearray(),'stderr':bytearray()}
    try:
        for name in buffers:
            fds.append(os.open(phase/(name+'.raw'),os.O_WRONLY|os.O_CREAT|os.O_EXCL,0o600))
        env=dict(PATH='/usr/bin:/bin',HOME=str(out/'home'),TMPDIR=str(out/'tmp'),LANG='C.UTF-8',LC_ALL='C.UTF-8',RUST_BACKTRACE='0')
        payload=req['stdin'].encode('utf-8')
        tool.require(len(payload)<=8193,'fixture stdin exceeds declared bound')
        native=subprocess.Popen(expected,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,env=env)
        observed=identity(native.pid)
        tool.write_new(phase/'native-identity.json',observed)
        if req['kind']=='parse-only' and req['argv'][1:]==['--validate-request']:
            tool.require(observed.get('uid')==os.getuid() and 'start' in observed,'parser exited before stdin identity')
        selector=selectors.DefaultSelector()
        for stream,name in [(native.stdout,'stdout'),(native.stderr,'stderr')]:
            os.set_blocking(stream.fileno(),False);selector.register(stream,selectors.EVENT_READ,name)
        os.set_blocking(native.stdin.fileno(),False);position=0;stdin_started=time.monotonic()
        if payload:selector.register(native.stdin,selectors.EVENT_WRITE,'stdin')
        else:native.stdin.close()
        while selector.get_map():
            if time.monotonic()>=deadline-.3:
                reason='native stream deadline';break
            for key,_ in selector.select(min(.03,max(0.,deadline-.3-time.monotonic()))):
                if key.data=='stdin':
                    try:position+=os.write(key.fd,payload[position:])
                    except BrokenPipeError:position=len(payload)
                    if position==len(payload):selector.unregister(key.fileobj);key.fileobj.close()
                    continue
                try:chunk=os.read(key.fd,8192)
                except BlockingIOError:continue
                if not chunk:selector.unregister(key.fileobj);key.fileobj.close();continue
                name=key.data;available=tool.MAX_RAW-len(buffers[name]);part=chunk[:max(0,available)]
                buffers[name].extend(part)
                if part:os.write(fds[0 if name=='stdout' else 1],part)
                if len(chunk)>available:overflow=True;reason='raw prefix bound exceeded';break
            if reason:break
        selector.close()
        if reason and native.returncode is None:native.kill()
        try:native.wait(timeout=max(.001,deadline-time.monotonic()-.15));reaped=True
        except subprocess.TimeoutExpired:reason=reason or 'native reap deadline'
    except Exception as error:
        reason=type(error).__name__+': '+str(error)
        if native is not None:
            native.kill()
            try:native.wait(timeout=max(.001,deadline-time.monotonic()-.1));reaped=True
            except subprocess.TimeoutExpired:pass
    finally:
        for fd in fds:
            try:os.fsync(fd)
            finally:os.close(fd)
    result={'name':req['name'],'returncode':native.returncode if native else None,'native_invoked':native is not None,
            'native_reaped':reaped,'raw_published':len(fds)==2,'truncated':overflow,
            'within_whole':time.monotonic()<=deadline and reason is None,
            'stdin_started_inside_whole':stdin_started is not None and started<=stdin_started<=deadline,
            'whole_started_monotonic':started,'deadline_monotonic':deadline,'reason':reason}
    tool.write_new(phase/'phase-result.json',result);tool.sync_dir(phase)
    tool.common_authority(root,packet)
    if time.monotonic()>deadline:reason=reason or 'final raw fsync exceeded original cap'
    if reason or not reaped:
        tool.write_new(phase/'refusal.json',{'first_red_monotonic':time.monotonic(),'reason':reason or 'reap unknown',
                       'artifact_published':True,'qualification_authorized':False})
        tool.sync_dir(phase);return 1
    return 0


if __name__=='__main__':
    try:raise SystemExit(main())
    except Exception as error:
        print(type(error).__name__+': '+str(error),file=sys.stderr);raise SystemExit(1)
