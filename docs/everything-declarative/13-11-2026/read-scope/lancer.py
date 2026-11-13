"""Lancement unique ; aucun Cargo avant le reçu du superviseur."""
import json, os, pathlib, subprocess, sys


def main():
    os.chdir('/var/tmp')
    w = pathlib.Path(sys.argv[1]).resolve(strict=True)
    source = sys.argv[2]
    assert len(source) == 40
    assert subprocess.check_output(['git', '-C', str(w), 'rev-parse', 'HEAD'], text=True).strip() == source
    assert not subprocess.check_output(['git', '-C', str(w), 'status', '--porcelain'])
    root = pathlib.Path.home() / '.cache/rag3weaver-build'
    p = root / 'cx-preuves-ed-read-scope-r2'
    t = root / 'cx-target-ed-read-scope-r2'
    assert not p.exists() and not t.exists(), 'pas de reprise aveugle'
    p.mkdir(mode=0o700)
    here = w / 'docs/everything-declarative/13-11-2026/read-scope'
    env = dict(os.environ, POSTE_MEM_MAX='32G')
    cmd = [sys.executable, str(w / 'extension/rag3weaver/scripts/confiner_validation.py'),
           'lancer', '--mode', 'mesure', '--delai', '6000', '--preuves', str(p / 'supervision'),
           '--', sys.executable, str(here / 'driver.py'), str(w), source, str(p), str(t)]
    (p / 'lancement.json').write_text(json.dumps(dict(source=source, commande=cmd), indent=2))
    with (p / 'supervision.log').open('w') as log:
        rc = subprocess.run(cmd, cwd='/var/tmp', env=env, stdout=log, stderr=subprocess.STDOUT).returncode
    return subprocess.run([sys.executable, str(here / 'finaliser.py'), str(w), source, str(p), str(t), str(rc)], cwd='/var/tmp').returncode


if __name__ == '__main__':
    sys.exit(main())
