"""Lancement unique ; aucun Cargo avant le reçu du superviseur."""
import json, os, pathlib, stat, subprocess, sys


def main():
    os.chdir('/var/tmp')
    w = pathlib.Path(sys.argv[1]).resolve(strict=True)
    source = sys.argv[2]
    assert len(source) == 40
    assert subprocess.check_output(['git', '-C', str(w), 'rev-parse', 'HEAD'], text=True).strip() == source
    assert not subprocess.check_output(['git', '-C', str(w), 'status', '--porcelain'])
    root = pathlib.Path.home() / '.cache/rag3weaver-build'
    p = root / 'cx-preuves-ed-read-scope-r4'
    t = root / 'cx-target-ed-read-scope-r4'
    assert not p.exists() and not t.exists(), 'pas de reprise aveugle'
    p.mkdir(mode=0o700)
    here = w / 'docs/everything-declarative/13-11-2026/read-scope'
    allowed = ('HOME', 'USER', 'LOGNAME', 'LANG', 'LC_ALL',
               'RAG3WEAVER_VALIDATION_RECU', 'RAG3WEAVER_VALIDATION_JETON')
    env = {k: os.environ[k] for k in allowed if k in os.environ}
    runtime = pathlib.Path('/run/user') / str(os.getuid())
    assert runtime.is_dir() and runtime.stat().st_uid == os.getuid() and not runtime.is_symlink()
    bus = runtime / 'bus'
    assert not bus.is_symlink() and bus.stat().st_uid == os.getuid() and stat.S_ISSOCK(bus.stat().st_mode)
    # Bus du gestionnaire utilisateur pour poste/systemd-run seulement.
    # Le confineur fournira ensuite son bus privé au driver et à ses enfants.
    env.update(XDG_RUNTIME_DIR=str(runtime), DBUS_SESSION_BUS_ADDRESS='unix:path=' + str(bus))
    env.update(PATH=str(pathlib.Path.home() / '.cargo/bin') + ':/usr/local/bin:/usr/bin:/bin', POSTE_MEM_MAX='32G', RAG3WEAVER_BUILD_MEMORY_HIGH='0')
    cmd = [str(w / 'extension/rag3weaver/scripts/poste'), 'lourd',
           sys.executable, str(here / 'apres-poste.py'), str(w), source, str(p), str(t)]
    (p / 'lancement.json').write_text(json.dumps(dict(source=source, commande=cmd), indent=2))
    with (p / 'supervision.log').open('w') as log:
        rc = subprocess.run(cmd, cwd='/var/tmp', env=env, stdout=log, stderr=subprocess.STDOUT).returncode
    return subprocess.run([sys.executable, str(here / 'finaliser.py'), str(w), source, str(p), str(t), str(rc)], cwd='/var/tmp').returncode


if __name__ == '__main__':
    sys.exit(main())
