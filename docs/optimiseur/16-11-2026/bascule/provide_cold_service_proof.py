#!/usr/bin/env python3
"""Observateur G borné hors Common : preuves API froide, aucune opération ni requête HTTP.

Contrat arbre 260d63d93f, stage service-guard-corrective-02. Réutilise les
primitives Common927 par une copie épinglée ; aucun jeton n'est écrit ou affiché.
"""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import time

COMMON_SHA = '6439fdd8b52c85fa4e74af0def2c667ef0bedae9b814cbbfd48e49946c4f74ea'
HOST_SHA = '0e310a9cfe1c9e12534f3a579b3e781dcef7e9335e37d7e57349b16d053d5288'
SOURCE_SHA = '927941a67a5805450d49f1e04b39fdd542a6059b'
STAGE = 'service-guard-corrective-02'


class Refusal(RuntimeError):
    pass


def require(ok, reason):
    if not ok:
        raise Refusal(reason)


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def normalize_cgroup(value):
    require(isinstance(value,str) and '\n' not in value, 'ambiguous cgroup')
    value = value.removeprefix('0::')
    require(value.startswith('/') and '..' not in Path(value).parts, 'invalid cgroup')
    return value


def parse_units(stdout, socket=False):
    result = {}
    for line in stdout.splitlines():
        if not line:
            continue
        require('=' in line, 'invalid unit output')
        key,value = line.split('=',1)
        require(key not in result, 'duplicate unit key')
        result[key] = value
    expected = dict(LoadState='loaded',ActiveState='inactive',SubState='dead')
    if not socket:
        expected['MainPID'] = '0'
    require(result == expected, 'unit output is not exactly loaded/inactive/dead/PID0')
    return result


def write_json(path, data):
    with path.open('x') as stream:
        json.dump(data,stream,indent=2)
        stream.flush()
        os.fsync(stream.fileno())


def identity(pid):
    proc = Path('/proc')/str(pid)
    fields = (proc/'stat').read_text().rsplit(') ',1)[1].split()
    require(fields[0] != 'Z', 'process is a zombie')
    return dict(pid=int(pid),start=fields[19],cgroup=(proc/'cgroup').read_text().strip(),
                uid=proc.stat().st_uid,comm=(proc/'comm').read_text().strip())


def validate_request(request, phase, guard, now):
    require(request['schema_version']==1 and request['stage']==STAGE, 'wrong request version/stage')
    require(request['phase']==phase and re.fullmatch('[0-9a-f]{32}',request['nonce']), 'wrong phase/nonce')
    require(0 < request['entered_monotonic'] <= request['requested_monotonic'] <= now, 'request clock order')
    require(now-request['requested_monotonic'] <= 30, 'stale request')
    expected = request['expected_host']
    require(expected['pid']==3761669 and expected['exe_sha256']==HOST_SHA
            and expected['source_sha']==SOURCE_SHA, 'wrong expected host/source')
    require(Path(expected['exe_path']) == Path.home()/'.local/bin/rag3weaver-backend', 'wrong host path')
    for field,name in [('reply_path',phase+'-response.json'),('reply_sha_path',phase+'-response.sha256')]:
        require(Path(request[field])==guard/name, 'reply path outside the phase')
    for field in ['canonical_base','target_base','source_snapshot']:
        path = Path(request[field])
        require(path.is_absolute() and path.resolve()==path, 'noncanonical path')
        require(not any(p in ('.env','.vault') for p in path.parts), 'protected path')
    require(request['canonical_base'] != request['target_base'], 'copy target is the live base')


def verify_common(request, common):
    meta = request['common_receipt']
    path = Path(meta['path'])
    require(not path.is_symlink() and path.name=='autorite.json', 'invalid Common receipt path')
    info = path.stat()
    require(info.st_uid==os.getuid() and not info.st_mode & 0o077, 'nonprivate Common receipt')
    require(digest(path)==meta['sha256'], 'Common receipt SHA mismatch')
    # Le jeton demeure exclusivement dans ce fichier ; ne jamais recopier son dictionnaire.
    receipt = json.loads(path.read_text())
    require(receipt['etat']=='actif' and time.monotonic()<receipt['echeance'], 'Common inactive/expired')
    require(receipt['pid']==meta['pid'] and receipt['debut']==str(meta['start'])
            and receipt['cgroup']==normalize_cgroup(meta['cgroup'])
            and receipt['echeance']==meta['deadline'], 'Common binding mismatch')
    table = common.processus()
    runner = request['runner']
    require(runner['pid'] in table and receipt['pid'] in table, 'runner/supervisor gone')
    for pid,start,cgroup in [(runner['pid'],runner['start'],runner['cgroup']),
                             (meta['pid'],meta['start'],meta['cgroup'])]:
        require(table[pid]['debut']==str(start) and table[pid]['cgroup']==normalize_cgroup(cgroup),
                'PID/start/cgroup mismatch')
    require(meta['pid'] in common.ancetres(runner['pid'],table), 'runner outside Common ancestry')
    require(common.sous(table[runner['pid']]['cgroup'],receipt['cgroup']), 'runner outside Common scope')
    require(common.signature_superviseur(meta['pid'],receipt), 'supervisor executable signature mismatch')
    common.plafond(table[runner['pid']]['cgroup'])
    require(not common.sous(table[os.getpid()]['cgroup'],receipt['cgroup']), 'provider inside Common')
    config = json.loads((path.parent/'recette.json').read_text())
    require(request['entered_monotonic'] >= receipt['echeance']-config['delai'], 'entry before Common')
    ancestors = list(common.ancetres(runner['pid'],table))
    held = False
    for pid in ancestors:
        try:
            note = (Path.home()/'.cache/rag3weaver-build/fiches'/str(pid)).read_text()
            argv = (Path('/proc')/str(pid)/'cmdline').read_bytes().split(b'\0')
            if note.startswith(('tient lourd ','tient mesure ')) and any(Path(os.fsdecode(x)).name=='poste' for x in argv if x):
                held = True
        except (OSError,ValueError):
            pass
    require(held, 'no actual held poste in runner ancestry')
    return receipt['echeance']


def scan_references(bases):
    def related(value):
        return any(value==str(base) or value.startswith(str(base)+'.')
                   or value==str(base.with_suffix('.checkpoints'))
                   or value.startswith(str(base.with_suffix('.checkpoints'))+'/') for base in bases)
    refs = dict(fd=[],maps=[])
    protected = []
    for proc in Path('/proc').iterdir():
        if not proc.name.isdigit():
            continue
        try:
            if proc.stat().st_uid != os.getuid():
                continue
            item = identity(int(proc.name))
            unreadable = False
            try:
                for fd in (proc/'fd').iterdir():
                    try:
                        value = os.readlink(fd)
                        if related(value):
                            refs['fd'].append(dict(pid=item['pid'],start=item['start'],path=value,fd=fd.name))
                    except FileNotFoundError:
                        pass
            except PermissionError:
                unreadable = True
            try:
                for line in (proc/'maps').read_text().splitlines():
                    fields = line.split(None,5)
                    if len(fields)==6 and related(fields[5]):
                        refs['maps'].append(dict(pid=item['pid'],start=item['start'],path=fields[5]))
            except PermissionError:
                unreadable = True
            if unreadable:
                protected.append({k:item[k] for k in ['pid','start','uid','comm']})
        except (FileNotFoundError,ProcessLookupError):
            pass
        except Refusal:
            # Un zombie n'a plus de descripteur ni de mapping : aucun ancien PID réutilisé admis.
            pass
    return refs,protected


def collect(request, phase, guard, common):
    raw = guard/(phase+'-g-raw')
    raw.mkdir(mode=0o700)
    files = []
    units = {}
    for unit,kind in [('rag3weaver-daemon.service','units_service'),('rag3weaver-daemon.socket','units_socket')]:
        properties = ['LoadState','ActiveState','SubState'] + ([] if kind=='units_socket' else ['MainPID'])
        argv = ['systemctl','--user','show',unit]
        for field in properties:
            argv += ['-p',field]
        result = subprocess.run(argv,capture_output=True,text=True,timeout=15)
        stdout = raw/(kind+'.stdout')
        stderr = raw/(kind+'.stderr')
        with stdout.open('x') as stream:
            stream.write(result.stdout)
        with stderr.open('x') as stream:
            stream.write(result.stderr)
        files += [dict(kind=kind,path=str(stdout),sha256=digest(stdout)),
                  dict(kind=kind+'_stderr',path=str(stderr),sha256=digest(stderr))]
        require(result.returncode==0, 'systemctl observation failed')
        units[unit] = parse_units(result.stdout,kind=='units_socket')
    expected = request['expected_host']
    require(not (Path('/proc')/str(expected['pid'])).exists(), 'previous host PID present')
    executable = Path(expected['exe_path'])
    require(digest(executable)==HOST_SHA, 'installed ELF changed')
    port = subprocess.run(['ss','-H','-ltnp','sport = :7970'],capture_output=True,text=True,timeout=15)
    port_raw = raw/'port.json'
    write_json(port_raw,dict(argv=['ss','-H','-ltnp','sport = :7970'],rc=port.returncode,
                             stdout=port.stdout,stderr=port.stderr))
    files.append(dict(kind='port',path=str(port_raw),sha256=digest(port_raw)))
    require(port.returncode==0 and not port.stdout.strip(), '7970 listener or probe error')
    refs,protected = scan_references([Path(request['canonical_base']),Path(request['target_base'])])
    scan = raw/'base-scan.json'
    write_json(scan,dict(base_refs=refs,protected_processes=protected))
    files.append(dict(kind='base_scan',path=str(scan),sha256=digest(scan)))
    require(not refs['fd'] and not refs['maps'], 'database references remain')
    pins = raw/'pins.json'
    write_json(pins,dict(path=str(executable),sha256=digest(executable),source_sha=SOURCE_SHA))
    files.append(dict(kind='pins',path=str(pins),sha256=digest(pins)))
    verify_common(request,common)
    observed = time.monotonic()
    require(observed >= request['requested_monotonic'], 'observation predates request')
    observer = identity(os.getpid())
    return dict(schema_version=1,stage=request['stage'],nonce=request['nonce'],phase=phase,
                entered_monotonic=request['entered_monotonic'],runner=request['runner'],
                common_receipt_sha256=request['common_receipt']['sha256'],observed_monotonic=observed,
                observer={k:observer[k] for k in ['pid','start','cgroup','uid']},verdict='VERT',units=units,
                previous_host=dict(pid=expected['pid'],absent=True),
                executable=dict(path=str(executable),sha256=HOST_SHA,source_sha=SOURCE_SHA),
                port_7970=dict(rc=0,listeners=[]),base_refs=refs,protected_processes=protected,
                limits=dict(uid_scope=os.getuid(),protected_processes_are_unknown=True,
                            no_http=True,no_unit_operation=True,provider_outside_common=True),
                raw_files=files,cli_authorized=False)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--guard',required=True,type=Path)
    parser.add_argument('--common-script',required=True,type=Path)
    parser.add_argument('--evidence',required=True,type=Path)
    parser.add_argument('--seconds',type=int,default=1800)
    args = parser.parse_args()
    os.umask(0o077)
    require(os.getuid()==1000, 'wrong observer UID')
    require(0 < args.seconds <= 1800, 'provider budget outside bound')
    require(digest(args.common_script)==COMMON_SHA, 'trusted Common source changed')
    spec = importlib.util.spec_from_file_location('trusted_common_927',args.common_script)
    common = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(common)
    guard = args.guard.absolute()
    require(guard.resolve()==guard and not any(x in ('.env','.vault') for x in guard.parts), 'linked/protected guard')
    args.evidence.mkdir(parents=True,exist_ok=False)
    write_json(args.evidence/'observer.json',dict(observer=identity(os.getpid()),budget_seconds=args.seconds,
                                               guard=str(guard),common_sha256=COMMON_SHA))
    end = time.monotonic()+args.seconds
    seen = set()
    receipt_path = None
    responded = []
    try:
        for phase in ['versions','generations']:
            request_path = guard/(phase+'-request.json')
            while not request_path.exists():
                require(time.monotonic()<end, 'provider deadline before phase')
                if receipt_path:
                    current = json.loads(receipt_path.read_text())
                    if current['etat']!='actif' or time.monotonic()>=current['echeance']:
                        write_json(args.evidence/'finished.json',dict(reason='Common ended',responded=responded))
                        return 0
                time.sleep(.05)
            require(not request_path.is_symlink() and request_path.stat().st_uid==os.getuid(), 'invalid request owner/type')
            request_bytes = request_path.read_bytes()
            request = json.loads(request_bytes)
            validate_request(request,phase,guard,time.monotonic())
            require(request['nonce'] not in seen, 'reused phase nonce')
            seen.add(request['nonce'])
            verify_common(request,common)
            receipt_path = Path(request['common_receipt']['path'])
            response = collect(request,phase,guard,common)
            require(request_path.read_bytes()==request_bytes, 'request changed during observation')
            response['request_sha256'] = hashlib.sha256(request_bytes).hexdigest()
            reply = Path(request['reply_path'])
            write_json(reply,response)
            # Le consommateur ne lit qu'après publication du SHA, jamais une réponse partielle.
            with Path(request['reply_sha_path']).open('x') as stream:
                stream.write(digest(reply)+'\n')
                stream.flush()
                os.fsync(stream.fileno())
            responded.append(phase)
            print(json.dumps(dict(phase=phase,reply=str(reply),sha256=digest(reply),observer_pid=os.getpid())),flush=True)
        while time.monotonic()<end:
            current = json.loads(receipt_path.read_text())
            if current['etat']!='actif' or time.monotonic()>=current['echeance']:
                break
            time.sleep(.1)
        write_json(args.evidence/'finished.json',dict(reason='Common ended or provider deadline',responded=responded))
        return 0
    except Exception as error:
        write_json(args.evidence/'refusal.json',dict(verdict='ROUGE',error=f'{type(error).__name__}: {error}',responded=responded))
        print(f'ROUGE provider: {type(error).__name__}: {error}',file=sys.stderr)
        return 1


if __name__=='__main__':
    raise SystemExit(main())
