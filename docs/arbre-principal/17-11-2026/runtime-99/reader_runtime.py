"""First real typed Reader invocation through the adapted GDB boundary.

All later queries use the unchanged qualified Reader. This attests this ONE
invocation, not maps for later processes or a plugin loaded after main.
"""
import io,json,math,os,subprocess,time
from pathlib import Path

def validate_capture(config,first,main,exited):
    for sample in [first,main]:
        if type(sample.get('pid')) is not int or sample['pid']<=0 or not str(sample.get('start','')).isdigit():raise ValueError('reader PID/start invalid')
        if sample.get('exe')!=config['argv'][0] or sample.get('exe_sha256')!=config['sha256'] or sample.get('cgroup')!=config['cgroup'] or sample.get('argv')!=config['argv']:raise ValueError('reader capture identity differs')
        if type(sample.get('rawlen')) is not int or sample['rawlen']<=0 or sample.get('expected')!=config['expected'] or sample.get('actual')!=config['expected'] or sample.get('diff')!={}:raise ValueError('reader captured profile differs')
        if sample.get('present')!={key:True for key in config['expected']}:raise ValueError('reader presence differs')
        if sample.get('after')!=dict(start=sample['start'],exe=sample['exe'],cgroup=sample['cgroup'],exe_sha256=sample['exe_sha256']):raise ValueError('reader after-sample changed')
    if any(first[key]!=main[key] for key in ['pid','start','inferior','exe','exe_sha256','cgroup','argv']):raise ValueError('EXEC/MAIN differ')
    native=[line for line in main['maps'].splitlines() if 'librag3db.so' in line]
    if not native or any(line.split()[-1]!=config['library'] for line in native) or main.get('native')!=native:raise ValueError('reader SDK map differs')
    if any(exited.get(key)!=main[key] for key in ['pid','start','inferior']) or type(exited.get('exit_code')) is not int or exited['exit_code']!=0:raise ValueError('real inferior exit not rc0')
    if not first['observed']<=main['observed']<=exited['observed']<config['deadline']:raise ValueError('reader observation deadline/order')

def first_query(reader,q,profile,sql,collect,callback=None):
    q.require_time(reader.end);reader.counter+=1;proof=q.P/'reader-sdk';proof.mkdir(mode=0o700)
    duration=min(90.,reader.end-time.monotonic()-15.);deadline=time.monotonic()+duration
    if not math.isfinite(deadline) or duration<=0:raise ValueError('no reader phase budget')
    cfg=dict(proof=str(proof),argv=[str(reader.binary),str(q.BASE),sql],sha256=q.sha(reader.binary),expected=profile,cgroup=Path('/proc/self/cgroup').read_text(),library=str(q.LIB/'librag3db.so'),deadline=deadline)
    q.save(proof/'config.json',cfg)
    helper=q.Q/'reader_capture.py';script=proof/'capture.gdb'
    body="set confirm off\nset pagination off\nset debuginfod enabled off\nset startup-with-shell off\nset follow-exec-mode same\nset print thread-events off\ncatch exec\nrun\npython\nimport json\nfrom pathlib import Path\nexec(compile(Path(HELPER).read_text(),HELPER,'exec'))\nconfig=json.loads(Path(CONFIG).read_text())\nfirst=capture('exec',config)\nend\ntbreak main\ncontinue\npython\nmain=capture('main',config)\ninstall_exit(config,main)\nend\ncontinue\n"
    body=body.replace('HELPER',repr(str(helper))).replace('CONFIG',repr(str(proof/'config.json')));script.write_text(body)
    command=[reader.timeout,'--signal=TERM','--kill-after=2s',str(duration)+'s',q.GDB,'--nx','--batch','--return-child-result','-x',str(script),'--args',q.PYTHON,'-B',str(q.Q/'reader_exec.py'),str(proof/'config.json')]
    q.save(proof/'command.json',dict(argv=command,deadline=deadline))
    with (proof/'gdb.stdout').open('xb') as out,(proof/'gdb.stderr').open('xb') as err:
        rc=subprocess.run(command,env=reader.env,stdout=out,stderr=err,cwd=q.P).returncode
    q.save(proof/'gdb-process.json',dict(rc=rc))
    # rc GDB alone is never the native result; link actual exited event to MAIN.
    if rc:raise ValueError('reader GDB invocation refused')
    records=[json.loads((proof/name).read_text()) for name in ['capture-exec.json','capture-main.json','native-exit.json']]
    validate_capture(cfg,*records);values=[]
    with (proof/'native.stdout').open('rb') as stream:result=q.typed_rows.consume(stream,values.append if collect else callback)
    if b'59ac9c86bc2b3529b590a7462ab7051e3ad26b8a' not in (proof/'native.stderr').read_bytes():raise ValueError('reader actual BUILD_ID absent')
    q.save(q.P/('read-'+str(reader.counter)+'.json'),dict(query=sql,rc=records[2]['exit_code'],rows=result['count'],columns=result['columns'],runtime_attestation=str(proof)))
    q.require_time(deadline,0);q.require_time(reader.end)
    return values if collect else result
