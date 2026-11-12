#!/usr/bin/env python3
"""Retrouve le CMake du moteur dans les messages du cargo test courant.

Ticket : docs/optimiseur/tickets/2026-11-12-la-ci-cherche-le-bati-vectoriel-dans-un-dossier-absent.md
Aucun glob de cache ni classement par date ; aucune compilation dans ce helper.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import sys


class Refus(ValueError):
    pass


def exiger(condition, raison):
    if not condition:
        raise Refus(raison)


def empreinte(path):
    h = hashlib.sha256()
    with path.open('rb') as f:
        for bloc in iter(lambda: f.read(1024 * 1024), b''):
            h.update(bloc)
    return h.hexdigest()


def revision(repo):
    return subprocess.check_output(['git', '-C', str(repo), 'rev-parse', '--verify', 'HEAD'], text=True).strip()


def paquet(metadata, repo, nom, manifeste):
    candidats = [p for p in metadata.get('packages', [])
                  if p.get('name') == nom and Path(p['manifest_path']).resolve() == repo / manifeste]
    exiger(len(candidats) == 1, f'paquet {nom} absent ou ambigu dans cargo metadata')
    return candidats[0]['id']


def contexte(repo, metadata_path):
    repo = repo.resolve(strict=True)
    metadata = json.loads(metadata_path.read_text())
    exiger(isinstance(metadata, dict), 'cargo metadata non objet')
    fichiers = ['tools/rust_api/build.rs', 'tools/rust_api/Cargo.toml',
                'extension/rag3weaver/Cargo.toml', 'extension/rag3weaver/Cargo.lock', 'CMakeLists.txt']
    return {
        'reason': 'rag3weaver-vector-ci-context', 'version': 1,
        'repo': str(repo), 'commit': revision(repo), 'metadata_sha256': empreinte(metadata_path),
        'producer_sha256': {p: empreinte(repo / p) for p in fichiers},
        'rag3db': paquet(metadata, repo, 'rag3db', 'tools/rust_api/Cargo.toml'),
        'rag3weaver': paquet(metadata, repo, 'rag3weaver', 'extension/rag3weaver/Cargo.toml'),
    }


def selectionner(repo, metadata_path, messages_path):
    repo = repo.resolve(strict=True)
    courant = contexte(repo, metadata_path)
    lignes = [json.loads(ligne) for ligne in messages_path.read_text().splitlines() if ligne.strip()]
    exiger(lignes and all(isinstance(x, dict) for x in lignes), 'messages Cargo absents ou invalides')
    exiger(lignes[0] == courant, 'contexte Cargo absent ou périmé (source, commit, recette ou metadata)')
    messages = lignes[1:]
    exiger(not any(x.get('reason') == courant['reason'] for x in messages), 'plusieurs invocations Cargo mélangées')
    fins = [x for x in messages if x.get('reason') == 'build-finished']
    exiger(len(fins) == 1 and messages[-1] == fins[0] and fins[0].get('success') is True,
           'bâti Cargo incomplet, ambigu ou en échec')
    scripts = [x for x in messages if x.get('reason') == 'build-script-executed'
               and x.get('package_id') == courant['rag3db']]
    exiger(len(scripts) == 1, 'build-script-executed rag3db absent ou ambigu')
    script = scripts[0]
    exiger(bool(script.get('out_dir')), 'OUT_DIR rag3db absent')
    out = Path(script['out_dir'])
    exiger(out.is_absolute(), 'OUT_DIR rag3db non absolu')
    out = out.resolve(strict=True)
    # cmake 0.1.57 prend OUT_DIR par défaut et configure OUT_DIR/build.
    # tools/rust_api/build.rs lie build_bundled_cmake().join("build/src").
    bdir = (out / 'build').resolve(strict=True)
    paths = [Path(p.removeprefix('native=')).resolve() for p in script.get('linked_paths', [])
             if p.startswith('native=')]
    exiger(bdir / 'src' in paths, 'le moteur lié ne provient pas du OUT_DIR CMake annoncé')
    exiger(any(p in script.get('linked_libs', []) for p in ['static=rag3db', 'static:+whole-archive=rag3db']),
           'le moteur lié n’est pas le moteur statique attendu')
    cache = bdir / 'CMakeCache.txt'
    valeurs = {}
    for ligne in cache.read_text().splitlines():
        if ligne.startswith(('#', '//')) or '=' not in ligne or ':' not in ligne.split('=', 1)[0]:
            continue
        cle, valeur = ligne.split('=', 1)
        cle = cle.split(':', 1)[0]
        exiger(cle not in valeurs, f'entrée CMake ambiguë : {cle}')
        valeurs[cle] = valeur
    for cle, attendu in [('CMAKE_HOME_DIRECTORY', repo), ('CMAKE_CACHEFILE_DIR', bdir)]:
        exiger(bool(valeurs.get(cle)) and Path(valeurs[cle]).resolve() == attendu,
               f'cache CMake périmé ou étranger : {cle}')
    archive = bdir / 'src/librag3db.a'
    exiger(archive.is_file() and archive.stat().st_size > 0, 'archive statique rag3db absente')
    moteurs = [x for x in messages if x.get('reason') == 'compiler-artifact'
               and x.get('package_id') == courant['rag3db'] and 'lib' in x.get('target', {}).get('kind', [])]
    exiger(len(moteurs) == 1, 'artefact Rust du moteur absent ou ambigu')
    exiger(any(Path(p).is_file() for p in moteurs[0].get('filenames', []) if p.endswith('.rlib')),
           'rlib du moteur annoncée mais absente')
    tests = [x for x in messages if x.get('reason') == 'compiler-artifact'
             and x.get('package_id') == courant['rag3weaver']
             and x.get('profile', {}).get('test') is True and x.get('executable')
             and 'lib' in x.get('target', {}).get('kind', [])]
    exiger(len(tests) == 1, 'exécutable des tests unitaires absent ou ambigu')
    test = tests[0]
    exiger({'rag3db-native', 'code', 'daemon', 'openai-llm'} <= set(test.get('features', [])),
           'features des tests différentes de la recette CI')
    executable = Path(test['executable']).resolve(strict=True)
    exiger(executable.is_file(), 'exécutable de tests absent')
    return {'context': courant, 'out_dir': str(out), 'cmake_build_dir': str(bdir),
            'cache_sha256': empreinte(cache), 'archive_sha256': empreinte(archive),
            'unit_test': str(executable), 'unit_test_sha256': empreinte(executable)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('mode', choices=['contexte', 'selectionner'])
    parser.add_argument('--repo', type=Path, required=True)
    parser.add_argument('--metadata', type=Path, required=True)
    parser.add_argument('--messages', type=Path)
    parser.add_argument('--rapport', type=Path)
    args = parser.parse_args()
    try:
        if args.mode == 'contexte':
            print(json.dumps(contexte(args.repo, args.metadata)))
        else:
            exiger(args.messages is not None and args.rapport is not None, '--messages et --rapport requis')
            resultat = selectionner(args.repo, args.metadata, args.messages)
            with args.rapport.open('x') as f:
                json.dump(resultat, f, indent=2)
                f.write('\n')
            print(json.dumps(resultat), file=sys.stderr)
            print(resultat['cmake_build_dir'])
    except (Refus, OSError, ValueError, KeyError, TypeError, subprocess.CalledProcessError) as e:
        print(f'ROUGE provenance vector : {e}', file=sys.stderr)
        return 1
    return 0


if __name__ == '__main__':
    sys.exit(main())
