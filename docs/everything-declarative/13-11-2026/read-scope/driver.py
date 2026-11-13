"""Cinq ciblés, module inventorié, frontières canoniques ; premier rouge arrêt."""
import hashlib, json, math, mmap, os, pathlib, re, shutil, subprocess, sys, time

FEATURES = 'rag3db-native,code,daemon'
MODULE = 'daemon_host::atelier_fichiers::'
PREFIX = MODULE + 'tests::'
JUGES = [
    'read_scope_preserves_a_backend_error_instead_of_claiming_absence',
    'read_scope_preserves_a_usages_error_instead_of_claiming_absence',
    'read_scope_refuses_unproven_uniqueness_after_a_real_definition',
    'read_scope_keeps_real_absence_and_ambiguity_after_complete_queries',
    'the_workshop_reads_a_scope_from_the_index_of_an_attached_worktree',
]
SDK = 'rag3db-build-29a7bc52aaf11e6204f9beafaffb154105417e13-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF'


def sha(p):
    with pathlib.Path(p).open('rb') as f:
        return hashlib.file_digest(f, 'sha256').hexdigest()


def write(p, value):
    p.write_text(json.dumps(value, indent=2, ensure_ascii=False) + '\n')


def main():
    os.chdir('/var/tmp')
    w, source, p, t = pathlib.Path(sys.argv[1]), sys.argv[2], pathlib.Path(sys.argv[3]), pathlib.Path(sys.argv[4])
    crate = w / 'extension/rag3weaver'
    receipt = pathlib.Path(os.environ['RAG3WEAVER_VALIDATION_RECU'])
    authority = json.loads(receipt.read_text())
    deadline = authority['echeance']
    rows, maps, disk_samples, unknown_maps = [], {}, [], []
    elf = None
    runtime_elf_sha = None
    def git(*args):
        return subprocess.check_output(['git', '-C', str(w), *args], text=True).strip()
    def propre():
        assert git('rev-parse', 'HEAD') == source
        assert not git('status', '--porcelain')
        assert not any(l[:1] in '-+U' for l in git('submodule', 'status', '--recursive').splitlines())
    propre()
    assert os.environ['RAG3WEAVER_BUILD_MEMORY_HIGH'] == '0'
    cg = pathlib.Path('/sys/fs/cgroup') / authority['cgroup'].lstrip('/')
    assert (cg / 'memory.max').read_text().strip() == str(32 * 1024**3)
    assert (cg / 'memory.high').read_text().strip() == 'max'
    write(p / 'source.json', dict(source=source, tree=git('rev-parse', 'HEAD^{tree}'), receipt=str(receipt), deadline=deadline,
                                  features=FEATURES, sdk=SDK, submodules=git('submodule', 'status', '--recursive')))
    (p / 'memory-before').write_text((cg / 'memory.events').read_text())
    assert not t.exists()
    allowed = ('HOME', 'USER', 'LOGNAME', 'PATH', 'LANG', 'LC_ALL',
               'RAG3WEAVER_VALIDATION_RECU', 'RAG3WEAVER_VALIDATION_JETON',
               'RAG3WEAVER_VERROU_TENU', 'RAG3WEAVER_BUILD_MEMORY_HIGH', 'DBUS_SESSION_BUS_ADDRESS')
    env = {k: os.environ[k] for k in allowed if k in os.environ}
    old_ld = None
    for name, sub in [('TMPDIR', 'tmp'), ('XDG_CACHE_HOME', 'cache'), ('XDG_CONFIG_HOME', 'config'),
                      ('XDG_DATA_HOME', 'data'), ('XDG_STATE_HOME', 'state'), ('RAG3WEAVER_STATE_DIR', 'state'),
                      ('RAG3WEAVER_JOURNAUX_DIR', 'commandes'), ('RAG3WEAVER_RUNS_DIR', 'runs'),
                      ('RAG3WEAVER_BUILTIN_TEMPLATES_DIR', 'templates'), ('RAG3WEAVER_CHECKPOINT_DIR', 'checkpoints')]:
        d = p / sub
        d.mkdir(exist_ok=True)
        env[name] = str(d)
    env.update(CARGO_TARGET_DIR=str(t), CARGO_BUILD_JOBS='8', CARGO_INCREMENTAL='0',
               RAG3DB_LIB=SDK, RAG3DB_LIBS_DIR=str(pathlib.Path.home() / '.cache/rag3db-libs'),
               RAG3DB_BUFFER_POOL_SIZE='1073741824')
    env.pop('RAG3WEAVER_TARGET_REPRENDRE', None)
    env.pop('RAG3WEAVER_MOTEUR_ANCIEN', None)
    for role in ['', '_EMBEDDER', '_RERANKER', '_OCR']:
        env['RAG3WEAVER_BURN_DEVICE' + role] = 'cpu'
    # Export natif unique dans ce profil ; pas de deuxième EXPORT autour des ELF.
    script = '. scripts/garde_target.sh; garder_le_target "$CARGO_TARGET_DIR" "$PWD" || exit; . scripts/lib_du_magasin.sh; exporter_la_lib || exit; exec python3 -c "import os,json; print(json.dumps(dict(os.environ)))"'
    env = json.loads(subprocess.check_output(['bash', '-c', script], cwd=crate, env=env, text=True, timeout=max(1, min(60, math.floor(deadline-time.monotonic()-60)))))
    write(p / 'profil.json', {k: v for k, v in env.items() if k != 'RAG3WEAVER_VALIDATION_JETON' and (k.startswith(('RAG3', 'CARGO_', 'XDG_')) or k in ('LD_LIBRARY_PATH', 'TMPDIR'))})
    sdk_so = pathlib.Path(env['RAG3DB_LIBRARY_DIR']) / 'librag3db.so'
    assert sdk_so.parent.name == SDK
    write(p / 'sdk.json', dict(path=str(sdk_so), sha256=sha(sdk_so)))
    def target_size(label):
        if not t.exists():
            return
        size = int(subprocess.check_output(['du', '-sb', '--', str(t)], cwd='/var/tmp', env=env, text=True, timeout=10).split()[0])
        disk_samples.append(dict(phase=label, bytes=size, monotonic=time.monotonic()))
        write(p / 'target-size.json', disk_samples)
        assert size <= 32 * 1024**3, 'target dépasse 32GiB ; aucun effacement correctif'
    def runtime_maps(label, proc):
        if not (label.startswith('cible-') or label == 'module') or label in maps:
            return
        q = pathlib.Path(f'/proc/{proc.pid}')
        try:
            st = (q / 'stat').read_text().rsplit(') ', 1)[1].split()
            exe = pathlib.Path(os.readlink(q / 'exe'))
            group = (q / 'cgroup').read_text().split('0::', 1)[1].strip()
            content = (q / 'maps').read_text()
            if exe != elf or not any(line.split()[-1:] == [str(sdk_so)] for line in content.splitlines()):
                return
            native_paths = {line.split()[-1] for line in content.splitlines() if '/librag3db' in line}
            assert native_paths == {str(sdk_so)}, ('SDK natif inattendu', native_paths)
            digest = sha(q / 'exe')
            again = (q / 'stat').read_text().rsplit(') ', 1)[1].split()
            if st[19] != again[19] or again[0] == 'Z':
                return
            assert group == authority['cgroup'] or group.startswith(authority['cgroup'] + '/')
            assert digest == runtime_elf_sha
            filename = f'maps-{label}-{proc.pid}-{st[19]}.txt'
            (p / filename).write_text(content)
            maps[label] = dict(phase=label, pid=proc.pid, starttime=st[19], exe=str(exe), cgroup=group,
                               elf_sha256=digest, sdk=str(sdk_so), sdk_sha256=sha(sdk_so), maps=filename,
                               observed=time.monotonic())
            write(p / 'maps-runtime.json', list(maps.values()))
        except (FileNotFoundError, ProcessLookupError):
            return
        except PermissionError as error:
            unknown_maps.append(dict(phase=label, pid=proc.pid, UNKNOWN=str(error), observed=time.monotonic()))
            write(p / 'maps-unknown.json', unknown_maps)
            raise RuntimeError('maps du juge attribué illisibles : preuve refusée') from error
    def red_signal(label):
        files = [p / (label + '.log')]
        if label == 'features':
            files += list((p / 'features').glob('*.log'))
        for path in files:
            if not path.exists():
                continue
            text = path.read_text(errors='replace')
            match = re.search(r'(?m)^test [^\n]*\.\.\. FAILED|^failures:|^CHECK_FEATURES ROUGE|^ROUGE |^error(?:\[E\d+\])?:|^thread [^\n]*panicked', text)
            if match:
                write(p / 'premier-rouge.json', dict(phase=label, log=str(path), signal=match.group(), observed=time.monotonic()))
                raise RuntimeError('premier rouge observé : arrêt attribué par superviseur ; ' + str(path))
    def run(label, argv, cap, runenv=None):
        seconds = min(cap, math.floor(deadline - time.monotonic() - 60))
        assert seconds > 0, 'budget global épuisé'
        record = dict(phase=label, argv=list(map(str, argv)), cwd=str(crate), timeout=seconds)
        rows.append(record)
        write(p / 'phases.json', rows)
        started = time.monotonic()
        target_size(label)
        last_disk = time.monotonic()
        with (p / (label + '.log')).open('w') as log:
            proc = subprocess.Popen(argv, cwd=crate, env=env if runenv is None else runenv, stdout=log, stderr=subprocess.STDOUT)
            while proc.poll() is None:
                assert time.monotonic() - started < seconds, 'timeout phase ; arrêt attribué par superviseur'
                runtime_maps(label, proc)
                if time.monotonic() - last_disk >= 10:
                    target_size(label)
                    last_disk = time.monotonic()
                red_signal(label)
                time.sleep(.05)
            record.update(rc=proc.returncode, elapsed_s=time.monotonic() - started)
        write(p / 'phases.json', rows)
        red_signal(label)
        target_size(label)
        assert record['rc'] == 0, label + ' rouge'
        return (p / (label + '.log')).read_text()
    run('compilation', ['ionice', '-c', '3', 'nice', '-n', '15', 'cargo', 'test', '--locked', '--lib', '--no-default-features', '--features', FEATURES,
                        '--no-run', '--message-format=json', '-j8'], 2400)
    artifacts = []
    for line in (p / 'compilation.log').read_text().splitlines():
        try:
            item = json.loads(line)
        except ValueError:
            continue
        if item.get('reason') == 'compiler-artifact' and item.get('executable') and item['profile'].get('test') and 'lib' in item['target']['kind']:
            artifacts.append(item)
    assert len(artifacts) == 1, 'ELF lib unique requis'
    item = artifacts[0]
    elf = pathlib.Path(item['executable']).resolve(strict=True)
    assert elf.is_relative_to(t.resolve())
    assert sorted(item['features']) == sorted(FEATURES.split(',')), item['features']
    archived = p / 'binaire-lib'
    shutil.copy2(elf, archived)
    runtime_elf_sha = sha(archived)
    assert runtime_elf_sha == sha(elf)
    with archived.open('rb') as f, mmap.mmap(f.fileno(), 0, access=mmap.ACCESS_READ) as data:
        source_offset = data.find(source[:9].encode())
    assert source_offset >= 0, 'source embarquée absente du véritable ELF'
    listing = run('inventaire-module', [str(elf), MODULE, '--list', '--format', 'terse'], 30)
    ignored = run('inventaire-ignores', [str(elf), MODULE, '--list', '--ignored', '--format', 'terse'], 30)
    names = [l.removesuffix(': test') for l in listing.splitlines() if l.endswith(': test')]
    ignores = [l.removesuffix(': test') for l in ignored.splitlines() if l.endswith(': test')]
    assert names and len(names) == len(set(names))
    expected_ignores = {PREFIX + n for n in [
        'the_permission_card_slides_under_the_call_and_grants_the_read',
        'the_command_card_grants_the_command_and_the_turn_starts_again_by_itself',
        'the_context_gauge_follows_each_model_call_says_its_percentage_and_survives_a_restart',
    ]}
    assert set(ignores) == expected_ignores, 'ignorés inattendus'
    assert set(PREFIX + n for n in JUGES) <= set(names) - set(ignores)
    # Publication hors source AVANT la première exécution d’un juge.
    ledger = dict(source=source, features=item['features'], executable=str(elf), copy=str(archived), source_offset=source_offset, sha256=sha(archived),
                  inventory=names, ignored=ignores, compiler_artifact=item, sdk_sha256=sha(sdk_so), before_runtime=time.monotonic())
    write(p / 'ledger-avant-runtime.json', ledger)
    def results(text, expected, nignore):
        matches = re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', text)
        assert matches and tuple(map(int, matches[-1])) == (expected, 0, nignore), matches
    for i, name in enumerate(JUGES):
        results(run(f'cible-{i+1}', [str(elf), PREFIX + name, '--exact', '--nocapture', '--test-threads=1'], 120), 1, 0)
    output = run('module', [str(elf), MODULE, '--nocapture', '--test-threads=8'], 600)
    results(output, len(names) - len(ignores), len(ignores))
    # --nocapture peut entrelacer les lignes par nom ; l'inventaire exact et
    # les comptes libtest font foi, sans reconstruire des statuts par ligne.
    assert maps, 'aucune observation SDK29 pendant un juge/module ; inventaires exclus'
    # Canonique : il exporte lui-même le SDK ; partir du LD avant notre export.
    checkenv = dict(env, CHECK_FEATURES_LOGS=str(p / 'features'), CHECK_FEATURES_TIMEOUT='1800')
    if old_ld is None:
        checkenv.pop('LD_LIBRARY_PATH', None)
    else:
        checkenv['LD_LIBRARY_PATH'] = old_ld
    text = run('features', ['ionice', '-c', '3', 'nice', '-n', '15', 'bash', 'scripts/check_features.sh'], 1800, checkenv)
    assert text.splitlines()[-1] == 'CHECK_FEATURES VERT 8/8'
    assert sha(archived) == ledger['sha256']
    original_present = elf.exists()
    if original_present:
        assert sha(elf) == ledger['sha256']
    write(p / 'elf-apres-features.json', dict(original_present=original_present, pruned=not original_present, copy_sha256=sha(archived)))
    propre()
    (p / 'memory-after').write_text((cg / 'memory.events').read_text())
    write(p / 'runtime.json', dict(verdict='VERT', source=source, elf_sha256=sha(archived), finished=time.monotonic(), deadline=deadline))


if __name__ == '__main__':
    try:
        main()
    except BaseException as e:
        if len(sys.argv) > 3:
            write(pathlib.Path(sys.argv[3]) / 'erreur-driver.json', dict(verdict='ROUGE', erreur=repr(e)))
        raise
