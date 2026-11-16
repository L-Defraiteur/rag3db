"""One scoped native pass; all raw logs remain under predeclared suite directories."""
import hashlib
import json
import os
from pathlib import Path
import re
import selectors
import shutil
import signal
import subprocess
import sys
import time
import xml.etree.ElementTree as ET

from parse_receipt import (DROP_CASE, excluded_scope, inspect_outcomes,
                           inspect_raw_identities, parse_gtest_inventory, verify_attestation, verify_e2e_partition)

scratch, out, reference = map(Path, sys.argv[1:4])
jobs, deadline = int(sys.argv[4]), float(sys.argv[5])
r = json.loads((out/'recipe.json').read_text())
source, build, tmp, home = [scratch/n for n in ['source','build','tmp','test-home']]
phase = 'preparation'
process = None
started = time.monotonic()
marker = None
raw = out/'raw'
for name in ['configure','build','offsets','transaction','known-red','strings','e2e-inventory','e2e-positive','e2e-drop','artifacts','parsing','staging']:
    (raw/name).mkdir(parents=True,exist_ok=True)
(out/'base').write_text(r['base']+'\n')
(out/'resume.log').write_text('commit '+r['source_commit']+'\n')

def write(path, data):
    path.write_text(json.dumps(data,indent=2)+'\n')

def sha(path):
    with path.open('rb') as f:
        return hashlib.file_digest(f,'sha256').hexdigest()

def group_exists(group):
    try:
        os.killpg(group,0)
        return True
    except ProcessLookupError:
        return False


def cgroup_pids():
    return {int(x) for x in (cgp/'cgroup.procs').read_text().split()}


def finish_child():
    if process is None:
        return False
    residue=process.poll() is not None and group_exists(process.pid)
    if group_exists(process.pid):
        try:
            os.killpg(process.pid,signal.SIGTERM)
        except ProcessLookupError:
            pass
        end=time.monotonic()+3
        while group_exists(process.pid) and time.monotonic()<end:
            process.poll()  # Reap an exited leader before interpreting killpg(0).
            if not group_exists(process.pid):
                break
            time.sleep(.05)
        if group_exists(process.pid):
            try:
                os.killpg(process.pid,signal.SIGKILL)
            except ProcessLookupError:
                pass
    if process.poll() is None:
        process.wait(timeout=3)
    return residue


def run(argv, label, seconds, env=None, allowed=(0,), attestation=False, cwd=None):
    global process
    process=None
    baseline_pids=cgroup_pids()
    directory=raw/label
    directory.mkdir(parents=True,exist_ok=True)
    remaining=deadline-time.monotonic()
    if remaining<=10:
        raise TimeoutError('global deadline before '+label)
    end=time.monotonic()+min(seconds,remaining-10)
    write(directory/'argv.json',argv)
    begin=time.monotonic();timed_out=False
    root_file=None
    with (directory/'stdout.log').open('wb') as log:
        try:
            process=subprocess.Popen(argv,stdout=subprocess.PIPE if attestation else log,
                                     stderr=subprocess.STDOUT,env=env,cwd=cwd,start_new_session=True)
            if attestation:
                root_file=(out/'attestation.log').open('xb')
                selector=selectors.DefaultSelector();selector.register(process.stdout,selectors.EVENT_READ)
                while selector.get_map():
                    if time.monotonic()>=end:
                        raise subprocess.TimeoutExpired(argv,end-begin)
                    for key,_ in selector.select(min(1,max(.01,end-time.monotonic()))):
                        data=os.read(key.fd,65536)
                        if data:
                            log.write(data);log.flush();root_file.write(data);root_file.flush()
                        else:
                            selector.unregister(key.fileobj)
                selector.close()
            code=process.wait(timeout=max(.01,end-time.monotonic()))
        except subprocess.TimeoutExpired:
            timed_out=True;finish_child();code=process.returncode
        finally:
            group_residue=finish_child()
            settle=time.monotonic()+3
            remaining_pids=cgroup_pids()-baseline_pids
            while remaining_pids and time.monotonic()<settle:
                time.sleep(.05)
                remaining_pids=cgroup_pids()-baseline_pids
            if root_file is not None:
                root_file.close()
            write(directory/'exit.json',{'code':process.returncode if process else None,
                  'timeout':timed_out,'pid':process.pid if process else None,
                  'group_residue_after_leader':group_residue,'remaining_phase_pids':sorted(remaining_pids),
                  'waited':process is not None and process.poll() is not None,
                  'elapsed_s':time.monotonic()-begin})
    if timed_out:
        raise TimeoutError(label)
    if group_residue or remaining_pids:
        raise RuntimeError(label+' left attributed descendants')
    if code not in allowed:
        raise RuntimeError(label+' returned '+str(code))
    return code

def confined(argv, testing=False, extensions_writable=True):
    cmd=['bwrap','--unshare-user','--unshare-net','--ro-bind','/','/','--proc','/proc','--dev','/dev',
         '--ro-bind',str(source),str(source)]
    if not testing:
        cmd+=['--bind',str(build),str(build)]
        if extensions_writable:
            for ext in ['vector','geo']:
                p=str(source/'extension'/ext/'build');cmd+=['--bind',p,p]
    else:
        cmd+=['--ro-bind',str(build),str(build)]
    cmd+=['--bind',str(tmp),str(tmp),'--bind',str(home),str(home),'--bind',str(out),str(out),
          '--chdir',str(tmp if testing else build)]
    if testing:
        cmd+=['/usr/bin/env','-i',*[k+'='+v for k,v in test_env.items()]]
    return cmd+argv

try:
    assert 1<=jobs<=8 and not any(scratch.iterdir())
    cg=next(x.split(':',2)[2] for x in Path('/proc/self/cgroup').read_text().splitlines() if x.startswith('0::'))
    cgp=Path('/sys/fs/cgroup')/cg.lstrip('/')
    q,p=(cgp/'cpu.max').read_text().split()
    admission=json.loads((raw/'driver/admission.json').read_text())
    assert q!='max' and int(q)/int(p)<=admission['child_cpu_quota_percent']/100+.00001
    assert jobs<=int(admission['poste_threads'])
    limits={'memory.max':(cgp/'memory.max').read_text().strip(),
            'memory.swap.max':(cgp/'memory.swap.max').read_text().strip(),
            'cpu.max':(cgp/'cpu.max').read_text().strip(),'jobs':jobs,
            'scratch_capacity_bytes':os.statvfs(scratch).f_blocks*os.statvfs(scratch).f_frsize,
            'routes':Path('/proc/net/route').read_text(),'started_monotonic':started}
    write(raw/'driver/limits.json',limits)
    assert limits['memory.max']=='34359738368' and limits['memory.swap.max']=='0'
    assert limits['scratch_capacity_bytes']==34359738368 and len(limits['routes'].splitlines())==1
    assert sha(out/'source.bundle')==r['bundle_sha256']
    for d in [build,tmp,home]:d.mkdir()
    os.environ.update({'HOME':str(home),'TMPDIR':str(tmp),'GIT_OPTIONAL_LOCKS':'0',
                       'GIT_TERMINAL_PROMPT':'0','GIT_ALLOW_PROTOCOL':'file','CCACHE_DISABLE':'1',
                       'CMAKE_BUILD_PARALLEL_LEVEL':str(jobs),'GIT_CONFIG_COUNT':'2',
                       'GIT_CONFIG_KEY_0':'gc.auto','GIT_CONFIG_VALUE_0':'0',
                       'GIT_CONFIG_KEY_1':'maintenance.auto','GIT_CONFIG_VALUE_1':'false'})
    run(['git','clone','--shared','--no-checkout',str(reference),str(source)],'staging/clone',60)
    run(['git','-C',str(source),'fetch',str(out/'source.bundle'),r['source_ref']],'staging/fetch',60)
    run(['git','-C',str(source),'checkout','--detach',r['source_commit']],'staging/checkout',60)
    actual=subprocess.check_output(['git','-C',str(source),'rev-parse','HEAD'],text=True).strip()
    assert actual==r['source_commit']
    assert not subprocess.check_output(['git','-C',str(source),'status','--porcelain'])
    for item in r['native_files']:
        assert sha(source/item['path'])==item['sha256']
    for ext in ['vector','geo']:(source/'extension'/ext/'build').mkdir(parents=True,exist_ok=True)
    lists=source/'test/transaction/concurrence'
    scope=excluded_scope(lists/'long.txt',lists/'probabilistic.txt')
    assert len(scope['long'])==29 and len(scope['probabilistic'])==17 and len(scope['union_not_run'])==45
    write(raw/'parsing/scope.json',scope)
    with (out/'resume.log').open('a') as summary:
        summary.write('scope bounded; long=29 probabilistic=17 overlap=1 union_not_run=45\n')
        for name in scope['union_not_run']:summary.write('NON JOUE '+name+'\n')
    test_env={'PATH':'/usr/bin:/bin','LANG':'C','LC_ALL':'C','TZ':'UTC','HOME':str(home),
              'TMPDIR':str(tmp),'RUNNER_TEMP':str(tmp),'MAX_NUM_THREADS':'2',
              'OMP_NUM_THREADS':str(jobs),'OPENBLAS_NUM_THREADS':str(jobs)}
    assert 'CONCURRENCE_LONG' not in test_env
    write(raw/'driver/test-env.json',test_env)
    phase='configure'
    config=['cmake','-S',str(source),'-B',str(build),'-G','Ninja','-DCMAKE_BUILD_TYPE=Release',
            '-DBUILD_TESTS=ON','-DBUILD_SHELL=OFF','-DBUILD_EXTENSIONS=vector;geo',
            '-DBUILD_SHARED_LIBS=OFF','-DENABLE_RUNTIME_CHECKS=OFF','-DENABLE_ADDRESS_SANITIZER=OFF',
            '-DENABLE_THREAD_SANITIZER=OFF','-DENABLE_UBSAN=OFF','-DENABLE_LTO=OFF',
            '-DENABLE_BACKTRACES=OFF','-DCMAKE_EXPORT_COMPILE_COMMANDS=ON','-DAUTO_UPDATE_GRAMMAR=OFF',
            '-DFETCHCONTENT_FULLY_DISCONNECTED=ON','-DFETCHCONTENT_UPDATES_DISCONNECTED=ON','-DCCACHE_PROGRAM:FILEPATH=']
    run(confined(config),'configure',120)
    phase='build'
    targets=['transaction_test','concurrence_test','e2e_test','dictionary_append_exception_test',
             'rag3db_vector_extension','rag3db_geo_extension']
    run(confined(['cmake','--build',str(build),'--target',*targets,'-j',str(jobs)]),'build',1800)
    phase='artifacts'
    files={'offsets':build/'test/transaction/dictionary_append_exception_test',
           'transaction':build/'test/transaction/transaction_test',
           'concurrence':build/'test/transaction/concurrence/concurrence_test',
           'e2e':build/'test/runner/e2e_test','engine.a':build/'src/librag3db.a',
           'vector':source/'extension/vector/build/libvector.rag3db_extension',
           'geo':source/'extension/geo/build/libgeo.rag3db_extension',
           'build_id.h':build/'src/include/common/build_id.h',
           'CMakeCache.txt':build/'CMakeCache.txt','compile_commands.json':build/'compile_commands.json'}
    inventory={}
    for name,path in files.items():
        if time.monotonic()>=deadline-10:raise TimeoutError('artifact export')
        shutil.copy2(path,raw/'artifacts'/name)
        inventory[name]={'path':str(path),'sha256':sha(path),'bytes':path.stat().st_size}
    marker=re.search(r'RAG3DB_BUILD_ID "([^"]+)"',(raw/'artifacts/build_id.h').read_text())[1]
    write(raw/'artifacts/manifest.json',{'source':actual,'engine':r['engine_commit'],
                                     'build_id_header':marker,'files':inventory})
    assert marker.startswith('rag3db-build-'+r['engine_commit']+'-') and '-dirty-' not in marker
    for name in ['offsets','transaction','concurrence','e2e','vector','geo']:
        assert marker.encode() in files[name].read_bytes()
    results={}
    identities={}
    phase='offsets'
    run(confined([str(files['offsets']),'--gtest_filter=DictionaryAppendException.OffsetAllocationFailureKeepsExistingStringsUnchanged',
                  '--gtest_color=no','--gtest_output=xml:'+str(raw/'offsets/result.xml')],True),
        'offsets',30,env=test_env,attestation=True)
    identities['offsets']=inspect_raw_identities(raw/'offsets/stdout.log',marker)
    results['offsets']=inspect_outcomes(raw/'offsets/result.xml','offsets')
    assert results['offsets']['executed']==1
    results['attestation']=verify_attestation(raw/'offsets/stdout.log',out/'attestation.log',marker)
    phase='transaction'
    run(confined([str(files['transaction']),'--gtest_color=no','--gtest_fail_fast',
                  '--gtest_output=xml:'+str(raw/'transaction/result.xml')],True),
        'transaction',2400,env=test_env)
    identities['transaction']=inspect_raw_identities(raw/'transaction/stdout.log',marker)
    results['transaction']=inspect_outcomes(raw/'transaction/result.xml','transaction')
    phase='known-red'
    # ctest needs to write discovery/result files under the private build; the actual
    # native engine/source/extensions stay RO. All such outputs are copied before judgment.
    command=['ctest','--test-dir',str(build/'test/transaction/concurrence'),'-R','^concurrence_test.known_red$',
             '--output-on-failure','--verbose']
    try:
        run(confined(command,False,False),'known-red',1800,env=test_env)
    finally:
        for path in (build/'test/transaction/concurrence').glob('concurrence_test_result*'):
            if path.is_file():shutil.copy2(path,raw/'known-red'/path.name)
    identities['known-red']=inspect_raw_identities(raw/'known-red/concurrence_test_result.json.log',marker)
    native=raw/'known-red/concurrence_test_result.json'
    result=json.loads(native.read_text())
    executed=[]
    for suite in result['testsuites']:
        for case in suite['testsuite']:
            name=suite['name']+'.'+case['name']
            assert case.get('result')=='COMPLETED', 'unexpected known-red skipped/not-run '+name
            executed.append(name)
    assert executed and not(set(executed)&set(scope['union_not_run']))
    control=(raw/'known-red/stdout.log').read_text()
    compare=(raw/'known-red/concurrence_test_result.json.log').read_text()
    assert 'INCOMPLETE: 29 long' in control and 'bench matches known_red.txt:' in control
    results['known-red']={'executed':len(executed),'json':result.get('tests'),
                          'long_not_run':29,'probabilistic_not_run':17,'union_not_run':45,
                          'comparator_exit':0}
    phase='strings'
    run(confined([str(files['concurrence']),'--gtest_filter=StringRewriteAcrossAddColumn.*',
                  '--gtest_color=no','--gtest_fail_fast','--gtest_output=xml:'+str(raw/'strings/result.xml')],True),
        'strings',600,env=test_env)
    identities['strings']=inspect_raw_identities(raw/'strings/stdout.log',marker)
    results['strings']=inspect_outcomes(raw/'strings/result.xml','strings')
    assert results['strings']['executed']==3
    e2e_end=min(time.monotonic()+1800,deadline-10)
    phase='e2e-inventory'
    run(confined([str(files['e2e']),'--gtest_filter=*','--gtest_list_tests','--gtest_color=no'],True),
        'e2e-inventory',min(30,e2e_end-time.monotonic()),env=test_env)
    identities['e2e-inventory']=inspect_raw_identities(raw/'e2e-inventory/stdout.log',marker)
    expected_cases=parse_gtest_inventory((raw/'e2e-inventory/stdout.log').read_text())
    write(raw/'parsing/e2e-inventory.json',sorted(expected_cases))
    phase='e2e-positive'
    run(confined([str(files['e2e']),'--gtest_filter=*-'+DROP_CASE,'--gtest_color=no',
                  '--gtest_fail_fast','--gtest_output=xml:'+str(raw/'e2e-positive/result.xml')],True),
        'e2e-positive',e2e_end-time.monotonic(),env=test_env)
    identities['e2e-positive']=inspect_raw_identities(raw/'e2e-positive/stdout.log',marker)
    results['e2e-positive']=inspect_outcomes(raw/'e2e-positive/result.xml','e2e-positive')
    positive_cases=[x.get('classname')+'.'+x.get('name') for x in ET.parse(raw/'e2e-positive/result.xml').getroot().iter('testcase')]
    assert len(positive_cases)==len(set(positive_cases))
    assert set(positive_cases)==expected_cases-{DROP_CASE}
    phase='e2e-drop'
    drop_code=run(confined([str(files['e2e']),'--gtest_filter='+DROP_CASE,'--gtest_color=no',
                           '--gtest_output=xml:'+str(raw/'e2e-drop/result.xml')],True),
                  'e2e-drop',e2e_end-time.monotonic(),env=test_env,allowed=(0,1))
    identities['e2e-drop']=inspect_raw_identities(raw/'e2e-drop/stdout.log',marker)
    drop_text=(raw/'e2e-drop/stdout.log').read_text()
    results['e2e-drop']=inspect_outcomes(raw/'e2e-drop/result.xml','e2e',drop_text)
    assert drop_code==bool(results['e2e-drop']['historical_failure_exceptions'])
    drop_cases=[x.get('classname')+'.'+x.get('name') for x in ET.parse(raw/'e2e-drop/result.xml').getroot().iter('testcase')]
    results['e2e-union']=verify_e2e_partition(expected_cases,positive_cases,drop_cases)
    for name,path in files.items():assert sha(path)==inventory[name]['sha256']
    assert not subprocess.check_output(['git','-C',str(source),'status','--porcelain'])
    write(raw/'parsing/receipt.json',{'source':actual,'engine':r['engine_commit'],'results':results,
                                    'identities':identities,'scope':scope,'installed':False})
    with (out/'resume.log').open('a') as log:
        log.write('SUITES ACCEPTED IN DECLARED BOUNDED SCOPE; awaiting quiescence\n')
except BaseException as exc:
    finish_child()
    write(raw/'parsing/first-failure.json',{'phase':phase,'type':type(exc).__name__,
          'message':str(exc),'elapsed_s':time.monotonic()-started,'retry':False})
    with (out/'resume.log').open('a') as log:log.write('FAILED phase '+phase+'\n')
    raise
finally:
    write(raw/'driver/scratch-final.json',{'capacity':os.statvfs(scratch).f_blocks*os.statvfs(scratch).f_frsize,
          'used':(os.statvfs(scratch).f_blocks-os.statvfs(scratch).f_bfree)*os.statvfs(scratch).f_frsize,
          'elapsed_s':time.monotonic()-started})
