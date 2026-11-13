"""Finalisation RO depuis /var/tmp ; aucun signal ni retrait de ressources."""
import hashlib, json, mmap, os, pathlib, subprocess, sys, time


def main():
    os.chdir('/var/tmp')
    w, source, p, t = pathlib.Path(sys.argv[1]), sys.argv[2], pathlib.Path(sys.argv[3]), pathlib.Path(sys.argv[4])
    result = dict(verdict='ROUGE', source=source, client_rc=int(sys.argv[5]))
    try:
        assert result['client_rc'] == 0
        runtime = json.loads((p / 'runtime.json').read_text())
        assert runtime['source'] == source and runtime['verdict'] == 'VERT'
        src = json.loads((p / 'source.json').read_text())
        receipt = pathlib.Path(src['receipt'])
        assert receipt.parent.is_relative_to(p / 'supervision')
        supervisor = json.loads((receipt.parent / 'verdict.json').read_text())
        assert supervisor['verdict'] == 'VERT' and supervisor['code'] == 0
        assert supervisor['restes'] == [] and supervisor['nettoye']
        assert not pathlib.Path(supervisor['prive']).exists()
        authority = json.loads(receipt.read_text())
        children = json.loads((receipt.parent / 'enfants.json').read_text())
        owned = {(str(r['pid']), str(r['debut'])) for r in children}
        owned.add((str(authority['pid']), str(authority['debut'])))
        found, unknown_external, unknown_blocking = [], [], []
        scope = authority['cgroup']
        for q in pathlib.Path('/proc').iterdir():
            if not q.name.isdigit() or int(q.name) == os.getpid():
                continue
            try:
                uid = q.stat().st_uid
                if uid != os.getuid() and not any(pid == q.name for pid, _ in owned):
                    continue
                stat = (q / 'stat').read_text().rsplit(') ', 1)[1].split()
                if stat[0] == 'Z':
                    continue
                row = dict(pid=q.name, starttime=stat[19], owned=(q.name, stat[19]) in owned)
                errors = {}
                for field in ('cgroup', 'exe', 'cwd'):
                    try:
                        row[field] = ((q / field).read_text().split('0::', 1)[1].strip()
                                      if field == 'cgroup' else os.readlink(q / field))
                    except PermissionError as e:
                        errors[field] = str(e)
                # Refuser un instantané mélangé après disparition/réemploi du PID.
                again = (q / 'stat').read_text().rsplit(') ', 1)[1].split()
                if again[19] != stat[19] or again[0] == 'Z':
                    continue
                cg = row.get('cgroup')
                in_scope = cg == scope or (cg is not None and cg.startswith(scope + '/'))
                exe, cwd = row.get('exe', ''), row.get('cwd', '')
                context = exe.startswith(str(t) + '/') or exe == str(p / 'binaire-lib') or cwd == str(w) or cwd.startswith(str(w) + '/')
                if row['owned'] or in_scope or context:
                    found.append(row)
                if errors:
                    row['UNKNOWN'] = errors
                    # Une baseline hors scope lisible reste externe, sans transformer
                    # ses fd/exe/cwd illisibles en observations négatives.
                    blocking = row['owned'] or in_scope or context or cg is None
                    (unknown_blocking if blocking else unknown_external).append(row)
            except (FileNotFoundError, ProcessLookupError):
                continue
            except PermissionError as e:
                # Si même l'identité/scope n'est pas lisible, l'exclusion est non prouvée.
                unknown_blocking.append(dict(pid=q.name, UNKNOWN=str(e)))
        quiet = dict(live=found, UNKNOWN_external=unknown_external, UNKNOWN_blocking=unknown_blocking)
        (p / 'quiescence.json').write_text(json.dumps(quiet, indent=2))
        assert not found and not unknown_blocking, 'quiescence attribuée non prouvée'
        assert subprocess.check_output(['git', '-C', str(w), 'rev-parse', 'HEAD'], text=True).strip() == source
        assert not subprocess.check_output(['git', '-C', str(w), 'status', '--porcelain'])
        ledger = json.loads((p / 'ledger-avant-runtime.json').read_text())
        copy = pathlib.Path(ledger['copy'])
        assert copy == p / 'binaire-lib'
        with copy.open('rb') as f:
            assert hashlib.file_digest(f, 'sha256').hexdigest() == ledger['sha256']
            with mmap.mmap(f.fileno(), 0, access=mmap.ACCESS_READ) as data:
                assert data.find(source[:9].encode()) == ledger['source_offset'] >= 0
        original = pathlib.Path(ledger['executable'])
        assert original.is_relative_to(t)
        after_elf = json.loads((p / 'elf-apres-features.json').read_text())
        assert original.exists() == after_elf['original_present']
        if original.exists():
            with original.open('rb') as f:
                assert hashlib.file_digest(f, 'sha256').hexdigest() == ledger['sha256']
        else:
            assert after_elf['pruned'], 'original absent sans preuve après features'
        assert time.monotonic() < src['deadline'], 'budget global dépassé'
        before = dict(l.split() for l in (p / 'memory-before').read_text().splitlines())
        after = dict(l.split() for l in (p / 'memory-after').read_text().splitlines())
        assert all(before[k] == after[k] for k in ('high', 'max', 'oom', 'oom_kill'))
        result.update(verdict='VERT', supervisor=supervisor, elf_sha256=ledger['sha256'], quiescence=quiet, original_present=original.exists())
    except BaseException as e:
        result['erreur'] = repr(e)
    finally:
        (p / 'verdict-compose.json').write_text(json.dumps(result, indent=2) + '\n')
        (p / 'VERDICT').write_text(result['verdict'] + '\n')
        manifest = []
        for f in sorted(p.rglob('*')):
            if not f.is_file() or f.is_symlink() or f.name == 'manifest.json':
                continue
            with f.open('rb') as stream:
                digest = hashlib.file_digest(stream, 'sha256').hexdigest()
            manifest.append(dict(path=str(f.relative_to(p)), size=f.stat().st_size, sha256=digest))
        (p / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
        print(json.dumps(result))
    return 0 if result['verdict'] == 'VERT' else 1


if __name__ == '__main__':
    sys.exit(main())
