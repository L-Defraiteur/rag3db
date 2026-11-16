"""Private dictionary append correction: one build, five cases, first failure stops."""
import hashlib
import json
import os
from pathlib import Path
import re
import signal
import subprocess
import sys
import time
import xml.etree.ElementTree as ET

scratch, out, reference, bundle, recipe_path = map(Path, sys.argv[1:])
recipe = json.loads(recipe_path.read_text())
start = time.monotonic()
deadline = start + 1770
phase = 'preparation'
source, build, tmp = scratch/'source', scratch/'build', scratch/'tmp'
process = None

def sha(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()

def write(name, value):
    (out/name).write_text(json.dumps(value, indent=2)+'\n')

def run(argv, name, seconds, env=None, cwd=None):
    global process
    remaining = deadline-time.monotonic()
    if remaining <= 10:
        raise TimeoutError('controller deadline before '+name)
    begun = time.monotonic()
    code = None
    expired = False
    with (out/name).open('wb') as log:
        process = subprocess.Popen(argv, stdout=log, stderr=subprocess.STDOUT,
                                   cwd=cwd, env=env, start_new_session=True)
        try:
            code = process.wait(timeout=min(seconds, remaining-10))
        except subprocess.TimeoutExpired:
            expired = True
            os.killpg(process.pid, signal.SIGTERM)
            try:
                process.wait(timeout=3)
            except subprocess.TimeoutExpired:
                os.killpg(process.pid, signal.SIGKILL)
                process.wait(timeout=3)
            code = process.returncode
        finally:
            write(name+'.result.json', {'argv': argv, 'pid': process.pid, 'returncode': code,
                  'timeout': expired, 'elapsed_s': time.monotonic()-begun,
                  'waited': process.poll() is not None})
    if expired:
        raise TimeoutError(name)
    if code:
        raise RuntimeError(name+' exit '+str(code))

try:
    cg = next(x.split(':', 2)[2] for x in Path('/proc/self/cgroup').read_text().splitlines() if x.startswith('0::'))
    root = Path('/sys/fs/cgroup')/cg.lstrip('/')
    limits = {'cgroup': cg, 'memory.max': (root/'memory.max').read_text().strip(),
              'memory.swap.max': (root/'memory.swap.max').read_text().strip(),
              'scratch_capacity_bytes': os.statvfs(scratch).f_blocks*os.statvfs(scratch).f_frsize,
              'routes': Path('/proc/net/route').read_text(), 'started_monotonic': start}
    write('limits.json', limits)
    assert limits['memory.max'] == '34359738368' and limits['memory.swap.max'] == '0'
    assert limits['scratch_capacity_bytes'] == 34359738368
    assert len(limits['routes'].splitlines()) == 1
    assert not any(scratch.iterdir()) and os.statvfs(reference).f_flag & os.ST_RDONLY
    assert sha(bundle) == recipe['bundle_sha256']
    tmp.mkdir(); build.mkdir()
    os.environ.update({'TMPDIR': str(tmp), 'GIT_OPTIONAL_LOCKS': '0', 'GIT_TERMINAL_PROMPT': '0',
                       'GIT_ALLOW_PROTOCOL': 'file', 'CCACHE_DISABLE': '1'})
    run(['git','clone','--shared','--no-checkout',str(reference),str(source)], 'clone.log', 60)
    run(['git','-C',str(source),'fetch',str(bundle),recipe['source_ref']], 'bundle.log', 60)
    run(['git','-C',str(source),'checkout','--detach',recipe['source_commit']], 'checkout.log', 60)
    actual = subprocess.check_output(['git','-C',str(source),'rev-parse','HEAD'],text=True).strip()
    assert actual == recipe['source_commit']
    assert not subprocess.check_output(['git','-C',str(source),'status','--porcelain'])
    def confined(argv):
        return ['bwrap','--unshare-user','--unshare-net','--ro-bind','/','/',
                '--proc','/proc','--dev','/dev','--ro-bind',str(source),str(source),
                '--bind',str(build),str(build),'--bind',str(tmp),str(tmp),
                '--bind',str(out),str(out),'--chdir',str(build),*argv]
    configure = ['cmake','-S',str(source),'-B',str(build),'-G','Ninja',
                 '-DCMAKE_BUILD_TYPE=Release','-DBUILD_TESTS=ON','-DBUILD_SHELL=OFF',
                 '-DBUILD_EXTENSIONS=','-DBUILD_SHARED_LIBS=OFF','-DENABLE_RUNTIME_CHECKS=OFF',
                 '-DENABLE_ADDRESS_SANITIZER=OFF','-DENABLE_THREAD_SANITIZER=OFF',
                 '-DENABLE_UBSAN=OFF','-DENABLE_LTO=OFF','-DENABLE_BACKTRACES=OFF',
                 '-DCMAKE_EXPORT_COMPILE_COMMANDS=ON','-DAUTO_UPDATE_GRAMMAR=OFF',
                 '-DFETCHCONTENT_FULLY_DISCONNECTED=ON','-DFETCHCONTENT_UPDATES_DISCONNECTED=ON',
                 '-DCCACHE_PROGRAM:FILEPATH=']
    phase = 'configure'
    run(confined(configure), 'configure.log', 120)
    for n in ['CMakeCache.txt','compile_commands.json']:
        (out/n).write_bytes((build/n).read_bytes())
    phase = 'build'
    run(confined(['cmake','--build',str(build),'--target','dictionary_append_exception_test','string_finalize_test','string_split_test','-j','8']),
        'build.log', min(1500, max(1,deadline-time.monotonic()-120)))
    phase = 'test_preparation'
    binary = build/'test/transaction/dictionary_append_exception_test'
    header = build/'src/include/common/build_id.h'
    text = header.read_text()
    marker = re.search(r'RAG3DB_BUILD_ID "([^"]+)"',text)[1]
    (out/'dictionary_append_exception_test').write_bytes(binary.read_bytes())
    extra_binaries = {}
    for target in ['string_finalize_test', 'string_split_test']:
        path = build/'test/storage'/target
        (out/target).write_bytes(path.read_bytes())
        extra_binaries[target] = {'sha256': sha(path), 'bytes': path.stat().st_size}
    (out/'build_id.h').write_text(text)
    archive = build/'src/librag3db.a'
    (out/'librag3db.a').write_bytes(archive.read_bytes())
    write('manifest.json', {'source': actual, 'expected_engine_commit': recipe['engine_commit'], 'build_id_header': marker, 'binary_sha256': sha(binary),
          'binary_bytes': binary.stat().st_size, 'archive_sha256': sha(archive),
          'archive_bytes': archive.stat().st_size, 'header_sha256': sha(out/'build_id.h'),
          'extra_binaries': extra_binaries,
          'distributed_install': False,
          'scratch_used_bytes': (os.statvfs(scratch).f_blocks-os.statvfs(scratch).f_bfree)*os.statvfs(scratch).f_frsize})
    assert marker.startswith('rag3db-build-'+recipe['engine_commit']+'-') and '-dirty-' not in marker
    assert marker.encode() in binary.read_bytes()
    env = {'PATH':'/usr/bin:/bin','LANG':'C','LC_ALL':'C','TZ':'UTC', 'TMPDIR':str(tmp)}
    write('test-environment.json',env)
    specifications = [
        (binary, 'DictionaryAppendException.OffsetAllocationFailureKeepsExistingStringsUnchanged', 'offsets', 1),
        (build/'test/storage/string_finalize_test', 'StringFinalizeTests.*', 'finalize', 2),
        (build/'test/storage/string_split_test', 'StringSplitTest.*', 'split', 2),
    ]
    results = []
    for executable, selection, label, expected_count in specifications:
        argv = [str(executable), '--gtest_filter='+selection, '--gtest_color=no',
                '--gtest_output=xml:'+str(out/(label+'.xml'))]
        cmd = ['bwrap','--unshare-user','--unshare-net','--ro-bind','/','/','--proc','/proc','--dev','/dev',
               '--ro-bind',str(source),str(source),'--ro-bind',str(build),str(build),
               '--bind',str(tmp),str(tmp),'--bind',str(out),str(out),'--chdir',str(tmp),
               '/usr/bin/env','-i',*[k+'='+v for k,v in env.items()],*argv]
        phase = 'test_'+label
        run(cmd,label+'.log',30,env=env)
        root_xml = ET.parse(out/(label+'.xml')).getroot()
        cases = list(root_xml.iter('testcase'))
        assert len(cases) == expected_count
        assert root_xml.get('errors') == '0' and root_xml.get('failures') == '0'
        for case in cases:
            assert case.get('status') == 'run' and case.get('result') == 'completed'
            assert case.find('skipped') is None and case.find('failure') is None
        log = (out/(label+'.log')).read_text()
        assert '[==========] Running '+str(expected_count)+' test' in log
        if label == 'offsets':
            assert 'BUILD_ID='+marker in log
        results.append({'label':label, 'xml':root_xml.attrib})
    write('result.json',{'verdict':'GREEN','suites':results,'cases':5,
                        'source':actual,'engine_commit':recipe['engine_commit'],
                        'build_id_emitted_by_offsets_target':marker,
                        'elapsed_s':time.monotonic()-start,'distributed_install':False})
except BaseException as exc:
    if process is not None and process.poll() is None:
        os.killpg(process.pid,signal.SIGTERM)
        try:
            process.wait(timeout=3)
        except subprocess.TimeoutExpired:
            os.killpg(process.pid,signal.SIGKILL);process.wait(timeout=3)
    write('first-failure.json',{'phase':phase,'type':type(exc).__name__,'message':str(exc),
          'elapsed_s':time.monotonic()-start,'retry':False,'source':recipe['source_commit']})
    raise
finally:
    write('scratch-final.json',{'capacity_bytes':os.statvfs(scratch).f_blocks*os.statvfs(scratch).f_frsize,
          'used_bytes':(os.statvfs(scratch).f_blocks-os.statvfs(scratch).f_bfree)*os.statvfs(scratch).f_frsize,
          'elapsed_s':time.monotonic()-start})
