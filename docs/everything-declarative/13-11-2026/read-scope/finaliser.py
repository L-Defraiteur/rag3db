"""Finalisation RO depuis /var/tmp ; aucun signal ni retrait de ressources."""
import hashlib, json, os, pathlib, subprocess, sys, time


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
        found, unknown = [], []
        for q in pathlib.Path('/proc').iterdir():
            if not q.name.isdigit() or int(q.name) == os.getpid():
                continue
            try:
                if q.stat().st_uid != os.getuid():
                    continue
                stat = (q / 'stat').read_text().rsplit(') ', 1)[1].split()
                if stat[0] == 'Z':
                    continue
                exe, cwd = os.readlink(q / 'exe'), os.readlink(q / 'cwd')
                if exe.startswith(str(t) + '/') or cwd == str(w) or cwd.startswith(str(w) + '/'):
                    found.append(dict(pid=q.name, starttime=stat[19], exe=exe, cwd=cwd))
            except (FileNotFoundError, ProcessLookupError):
                continue
            except PermissionError as e:
                unknown.append(dict(pid=q.name, refus=str(e)))
        (p / 'quiescence.json').write_text(json.dumps(dict(live=found, unknown=unknown), indent=2))
        assert not found and not unknown, 'quiescence non prouvée'
        assert subprocess.check_output(['git', '-C', str(w), 'rev-parse', 'HEAD'], text=True).strip() == source
        assert not subprocess.check_output(['git', '-C', str(w), 'status', '--porcelain'])
        ledger = json.loads((p / 'ledger-avant-runtime.json').read_text())
        with pathlib.Path(ledger['executable']).open('rb') as f:
            assert hashlib.file_digest(f, 'sha256').hexdigest() == ledger['sha256']
        assert time.monotonic() < src['deadline'], 'budget global dépassé'
        before = dict(l.split() for l in (p / 'memory-before').read_text().splitlines())
        after = dict(l.split() for l in (p / 'memory-after').read_text().splitlines())
        assert all(before[k] == after[k] for k in ('high', 'max', 'oom', 'oom_kill'))
        result.update(verdict='VERT', supervisor=supervisor, elf_sha256=ledger['sha256'], quiescence=dict(live=found, unknown=unknown))
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
