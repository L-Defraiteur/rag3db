#!/usr/bin/env python3
"""Inspecte avant retrait ; aucune sauvegarde, mutation Git ou suppression."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import stat
import subprocess
import sys


class Refus(Exception):
    pass


def inside(path, root):
    return path == root or root in path.parents


def git(path, *args, admin=False):
    env = dict(os.environ, GIT_OPTIONAL_LOCKS='0')
    # Ne pas laisser un dépôt sélectionné dans l'environnement remplacer l'argument.
    for key in ('GIT_DIR', 'GIT_WORK_TREE', 'GIT_COMMON_DIR', 'GIT_INDEX_FILE'):
        env.pop(key, None)
    command = ['git', '--git-dir' if admin else '-C', str(path), '-c', 'gc.auto=0', *args]
    result = subprocess.run(command, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    if result.returncode:
        raise Refus(f'inspection Git impossible : {path}')
    return result.stdout


def gitpath(path, name, admin=False):
    raw = os.fsdecode(git(path, 'rev-parse', '--path-format=absolute', name, admin=admin)).rstrip('\n')
    return Path(raw).resolve(strict=True)


def worktrees(common):
    rows = git(common, 'worktree', 'list', '--porcelain', '-z', admin=True).split(b'\0')
    entries = []
    for row in rows:
        if row.startswith(b'worktree '):
            p = Path(os.fsdecode(row[9:]))
            if not p.exists():
                raise Refus(f'autre worktree manquant ou ambigu : {p}')
            resolved = p.resolve(strict=True)
            # Git peut nommer l'administration principale d'un sous-module
            # au lieu du core.worktree ; lire sa racine effective.
            if resolved == common:
                resolved = gitpath(common, '--show-toplevel', admin=True)
            entries.append(resolved)
        elif row.startswith((b'prunable', b'locked')):
            raise Refus(f'entrée worktree ambiguë ou verrouillée : {common}')
    if not entries:
        raise Refus(f'inventaire worktree vide : {common}')
    return entries


def walk_error(error):
    raise error


def inventory(root):
    """Empreinte fidèle des entrées, sans contenu dans le rapport ni suivi des liens."""
    result = {}
    for directory, dirs, files in os.walk(root, followlinks=False, onerror=walk_error):
        for name in ['.'] + sorted(dirs + files):
            p = Path(directory) if name == '.' else Path(directory) / name
            rel = str(p.relative_to(root))
            info = p.lstat()
            if stat.S_ISLNK(info.st_mode) or not (stat.S_ISREG(info.st_mode) or stat.S_ISDIR(info.st_mode)):
                raise Refus(f'administration avec lien ou entrée spéciale : {root}')
            if name.endswith('.lock') or name in ('alternates', 'http-alternates', '.env', '.vault'):
                raise Refus(f'administration active ou dépendance externe ambiguë : {root}')
            digest = None
            if stat.S_ISREG(info.st_mode):
                h = hashlib.sha256()
                with p.open('rb') as stream:
                    for data in iter(lambda: stream.read(1024 * 1024), b''):
                        h.update(data)
                digest = h.hexdigest()
            result[rel] = (info.st_mode, info.st_uid, info.st_gid, info.st_mtime_ns, digest)
    return result


def inspect(repo, target, archives, report):
    refus = []
    target_admin = gitpath(target, '--absolute-git-dir')
    common = gitpath(target, '--git-common-dir')
    if gitpath(repo, '--git-common-dir') != common:
        raise Refus('cible étrangère au dépôt sélectionné')
    if target_admin == common or target not in worktrees(common):
        raise Refus('cible principale ou non enregistrée : retrait refusé')
    if gitpath(target, '--show-toplevel') != target:
        raise Refus('la cible ne désigne pas une racine de worktree')
    # Git retire le worktree et son administration privée, pas le common-dir extérieur.
    destroyed = [target, target_admin]
    if any(inside(report, p) for p in destroyed):
        raise ValueError('rapport situé dans la zone supprimable')
    admins = {target_admin}
    sources = {target}
    for directory, dirs, files in os.walk(target, followlinks=False, onerror=walk_error):
        p = Path(directory)
        if '.vault' in dirs:
            refus.append('zone .vault exclue : inspection structurelle incomplète')
            dirs.remove('.vault')
        for name in list(dirs):
            child = p / name
            if child.is_symlink():
                refus.append(f'répertoire lié non inspectable : {child}')
                dirs.remove(name)
        if '.git' in dirs or '.git' in files:
            entry = p / '.git'
            if entry.is_symlink():
                refus.append(f'gitfile lié ambigu : {entry}')
            else:
                try:
                    admin = gitpath(p, '--absolute-git-dir')
                    # Un .git cassé ne doit pas retomber sur le dépôt parent.
                    if gitpath(p, '--show-toplevel') != p.resolve():
                        raise Refus(f'gitfile cassé ou ambigu : {entry}')
                    admins.add(admin)
                    sources.add(p.resolve())
                except (Refus, OSError) as exc:
                    refus.append(str(exc))
            if '.git' in dirs:
                dirs.remove('.git')
    # Inclure les administrations résiduelles : même sans .git visible dans les sources.
    for root in list(admins):
        if not any(inside(root, d) for d in destroyed):
            continue
        for directory, dirs, files in os.walk(root, followlinks=False, onerror=walk_error):
            p = Path(directory)
            if 'HEAD' in files:
                admins.add(p)
            elif 'commondir' in files or ('config' in files and 'objects' in dirs):
                refus.append(f'administration résiduelle sans HEAD : {p}')
            dirs[:] = [n for n in dirs if n not in ('objects', 'logs', 'refs', 'hooks')]
    for source in sorted(sources):
        try:
            if git(source, 'status', '--porcelain=v1', '--untracked-files=all', '--ignored=matching'):
                refus.append(f'fichiers ou index non préservés dans les sources : {source}')
        except Refus as exc:
            refus.append(str(exc))
    descriptions = []
    for admin in sorted(admins):
        try:
            c = gitpath(admin, '--git-common-dir', admin=True)
            entries = worktrees(c)
            descriptions.append({'administration': str(admin), 'commune': str(c),
                                 'worktrees': list(map(str, entries))})
            if any(inside(c, d) for d in destroyed) and len(entries) > 1:
                refus.append(f'administration sert un autre worktree : {c}')
            for entry in entries:
                # Une dépendance externe serait détruite par le retrait, archive ou non.
                if any(inside(c, d) for d in destroyed) and not inside(entry, target):
                    refus.append(f'administration sert un autre worktree : {entry}')
        except (Refus, OSError) as exc:
            refus.append(str(exc))
    roots = {a for a in admins if any(inside(a, d) for d in destroyed)}
    roots = {a for a in roots if not any(a != b and inside(a, b) for b in roots)}
    evidence = []
    for root in sorted(roots):
        copy = archives.get(root)
        if copy is None:
            refus.append(f'archive fidèle absente : {root}')
            continue
        if any(inside(copy, d) or inside(d, copy) for d in destroyed) or any(inside(report, p) for p in (root, copy)):
            refus.append(f'archive ou rapport dans une zone dépendante : {root}')
            continue
        try:
            before = inventory(root)
            saved = inventory(copy)
            after = inventory(root)
            if before != after:
                refus.append(f'administration a changé pendant inspection : {root}')
            elif before != saved:
                refus.append(f'archive diffère de l’administration : {root}')
            else:
                summary = hashlib.sha256(json.dumps(before, sort_keys=True).encode()).hexdigest()
                evidence.append({'administration': str(root), 'archive': str(copy),
                                 'empreinte_sha256': summary, 'entrees': len(before)})
        except OSError:
            refus.append(f'administration ou archive inaccessible : {root}')
        except Refus as exc:
            refus.append(str(exc))
    for root in archives:
        if root not in roots:
            refus.append(f'preuve archive non attribuable : {root}')
    return {'verdict': 'ROUGE' if refus else 'VERT', 'worktree': str(target),
            'administrations': descriptions, 'archives': evidence, 'refus': sorted(set(refus)),
            'limite': 'Quiescence à la charge du propriétaire ; aucune suppression autorisée après modification.'}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--depot', required=True, type=Path)
    parser.add_argument('--worktree', required=True, type=Path)
    parser.add_argument('--rapport', required=True, type=Path)
    parser.add_argument('--archive', nargs=2, action='append', default=[], metavar=('ADMINISTRATION', 'COPIE'))
    args = parser.parse_args()
    try:
        target = args.worktree.resolve(strict=True)
        if Path(os.path.abspath(args.worktree)) != target:
            raise ValueError('chemin de cible lié ou ambigu')
        repo = args.depot.resolve(strict=True)
        report = args.rapport.absolute()
        if report.exists() or report.is_symlink():
            raise ValueError('rapport déjà présent : création exclusive exigée')
        report = report.parent.resolve(strict=True) / report.name
        if inside(report, target):
            raise ValueError('rapport situé dans la cible')
        if inside(report, gitpath(repo, '--git-common-dir')):
            raise ValueError('rapport situé dans une administration Git')
        archives = {}
        for source, copy in args.archive:
            key = Path(source).resolve(strict=True)
            if key in archives:
                raise ValueError('administration archive dupliquée')
            archives[key] = Path(copy).resolve(strict=True)
            if inside(report, key) or inside(report, archives[key]):
                raise ValueError('rapport situé dans une administration ou sa copie')
        try:
            result = inspect(repo, target, archives, report)
        except Refus as exc:
            result = {'verdict': 'ROUGE', 'refus': [str(exc)]}
        with report.open('x', encoding='utf-8') as stream:
            json.dump(result, stream, ensure_ascii=False, indent=2)
            stream.write('\n')
        print(result['verdict'])
        return 0 if result['verdict'] == 'VERT' else 2
    except (ValueError, OSError) as exc:
        print(f'ERREUR : {exc}', file=sys.stderr)
        return 1


if __name__ == '__main__':
    sys.exit(main())
