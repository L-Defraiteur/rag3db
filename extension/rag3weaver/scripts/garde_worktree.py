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


def protected(path):
    if any(part in ('.env', '.vault') for part in Path(path).parts):
        raise ValueError('chemin protégé : inspection refusée')


def safe_path(path, strict=True):
    protected(path)
    resolved = Path(path).resolve(strict=strict)
    protected(resolved)
    if Path(os.path.abspath(path)) != resolved:
        raise ValueError('racine liée ou ambiguë : inspection refusée')
    return resolved


def git(path, *args, admin=False):
    safe_path(path)
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
    return safe_path(raw)


def config_file(owner, path, admin=False):
    """Lire uniquement ce fichier, sans suivre des includes vers d'autres lieux."""
    path = safe_path(path)
    raw = git(owner, 'config', '--file', str(path), '--no-includes', '--null', '--list', admin=admin)
    result = {}
    for row in raw.split(b'\0'):
        if not row:
            continue
        key, sep, value = row.partition(b'\n')
        key = os.fsdecode(key)
        if key.startswith(('include.', 'includeif.')):
            raise Refus(f'configuration incluse ambiguë : {path}')
        result.setdefault(key, []).append(os.fsdecode(value) if sep else None)
    return result


def single(config, key):
    values = config.get(key, [])
    if len(values) > 1:
        raise Refus(f'configuration multiple ambiguë : {key}')
    return values[0] if values else None


def anchor(source, admin):
    """Une source est ancrée par sa propre entrée .git, jamais par le cwd de Git."""
    source = safe_path(source)
    entry = source / '.git'
    if not entry.exists() or entry.is_symlink():
        raise Refus(f'worktree sans ancrage .git : {source}')
    if gitpath(source, '--absolute-git-dir') != admin or gitpath(source, '--show-toplevel') != source:
        raise Refus(f'worktree et administration discordants : {source}')
    return source


def residual_module(common, sources):
    """Prouver le gitlink désinitialisé et son rangement privé, pas une absence seule."""
    matches = []
    for parent in sorted(sources):
        modules = parent / '.gitmodules'
        if not modules.exists():
            continue
        config = config_file(parent, modules)
        parent_admin = gitpath(parent, '--absolute-git-dir')
        for key, values in config.items():
            if not key.startswith('submodule.') or not key.endswith('.path'):
                continue
            name = key[len('submodule.'):-len('.path')]
            if not name or Path(name).is_absolute() or '..' in Path(name).parts:
                raise Refus(f'nom de sous-module ambigu : {parent}')
            stored = parent_admin / 'modules' / name
            if Path(os.path.abspath(stored)) != common:
                continue
            if len(values) != 1 or not values[0]:
                raise Refus(f'chemin de sous-module ambigu : {parent}')
            relative = values[0]
            if Path(relative).is_absolute() or '..' in Path(relative).parts:
                raise Refus(f'chemin de sous-module ambigu : {parent}')
            source = safe_path(parent / relative, strict=False)
            if not inside(source, parent) or source == parent:
                raise Refus(f'sous-module hors de sa source : {parent}')
            rows = [r for r in git(parent, 'ls-files', '--stage', '-z', '--', ':(literal)' + relative).split(b'\0') if r]
            if len(rows) != 1 or b'\t' not in rows[0]:
                raise Refus(f'gitlink de sous-module absent ou ambigu : {source}')
            metadata, path = rows[0].split(b'\t', 1)
            mode, oid, stage = metadata.split()
            if mode != b'160000' or stage != b'0' or path != os.fsencode(relative):
                raise Refus(f'gitlink de sous-module non attesté : {source}')
            if source.exists() and (not source.is_dir() or any(source.iterdir())):
                raise Refus(f'source désinitialisée non vide ou ambiguë : {source}')
            matches.append(dict(source_attendue=str(source), parent=str(parent), nom=name, gitlink=oid.decode('ascii')))
    if len(matches) != 1:
        raise Refus(f'administration sans source principale attestée : {common}')
    return matches[0]


def worktree_state(common, sources=()):
    """Reconstruire l'inventaire depuis les ancrages et inscriptions réciproques."""
    config = config_file(common, common / 'config', admin=True)
    extension = single(config, 'extensions.worktreeconfig')
    if 'extensions.worktreeconfig' in config and extension not in ('false', 'no', 'off', '0'):
        extra = common / 'config.worktree'
        if extra.exists():
            config.update(config_file(common, extra, admin=True))
    bare = single(config, 'core.bare')
    if bare not in ('false', 'no', 'off', '0'):
        raise Refus(f'administration nue ou état principal ambigu : {common}')
    candidates = {source for source in sources if gitpath(source, '--absolute-git-dir') == common}
    configured = single(config, 'core.worktree')
    if 'core.worktree' in config:
        if not configured:
            raise Refus(f'core.worktree vide ou ambigu : {common}')
        path = Path(configured)
        candidates.add(safe_path(path if path.is_absolute() else common / path))
    elif common.name == '.git':
        candidates.add(safe_path(common.parent))
    if len(candidates) > 1:
        raise Refus(f'ancrages de source principale discordants : {common}')
    primary = anchor(next(iter(candidates)), common) if candidates else None
    entries = [primary] if primary else []
    registrations = common / 'worktrees'
    linked = []
    if registrations.exists():
        safe_path(registrations)
        for item in sorted(registrations.iterdir()):
            item = safe_path(item)
            if not item.is_dir() or (item / 'locked').exists():
                raise Refus(f'inscription worktree ambiguë ou verrouillée : {item}')
            backlink = safe_path(item / 'gitdir')
            raw = os.fsdecode(backlink.read_bytes()).rstrip('\n')
            gitfile = Path(raw)
            if not gitfile.is_absolute():
                raise Refus(f'inscription worktree non absolue : {item}')
            gitfile = safe_path(gitfile)
            if gitfile.name != '.git' or not gitfile.is_file() or gitpath(item, '--git-common-dir', admin=True) != common:
                raise Refus(f'inscription worktree sans réciprocité : {item}')
            source = anchor(gitfile.parent, item)
            if source in entries:
                raise Refus(f'inscription worktree dupliquée : {item}')
            entries.append(source)
            linked.append(dict(administration=str(item), source=str(source)))
    if primary:
        return dict(etat='initialise', worktrees=entries, ancrage_principal=str(primary), inscriptions=linked)
    proof = residual_module(common, sources)
    if linked:
        raise Refus(f'administration désinitialisée sert un autre worktree : {common}')
    return dict(etat='desinitialise', worktrees=[], preuve_gitlink=proof, inscriptions=[])


def worktrees(common):
    return worktree_state(common)['worktrees']


def walk_error(error):
    raise error


def inventory(root):
    """Empreinte fidèle des entrées, sans contenu dans le rapport ni suivi des liens."""
    root = safe_path(root)
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


def source_link(path):
    """Lire le lien lui-même et le blob d'index, jamais ouvrir sa cible."""
    before = path.lstat()
    source = gitpath(path.parent, '--show-toplevel')
    relative = str(path.relative_to(source))
    rows = git(source, 'ls-files', '--stage', '-z', '--', ':(literal)' + relative).split(b'\0')
    rows = [row for row in rows if row]
    if len(rows) != 1 or b'\t' not in rows[0]:
        raise Refus(f'lien source non suivi ou ambigu : {path}')
    metadata, name = rows[0].split(b'\t', 1)
    mode, oid, stage = metadata.split()
    if mode != b'120000' or stage != b'0' or name != os.fsencode(relative):
        raise Refus(f'lien source hors mode 120000 stable : {path}')
    link = os.readlink(path)
    if git(source, 'cat-file', 'blob', oid.decode('ascii')) != os.fsencode(link):
        raise Refus(f'lien source modifié : {path}')
    after = path.lstat()
    fields = ('st_dev', 'st_ino', 'st_mode', 'st_size', 'st_mtime_ns', 'st_ctime_ns')
    if any(getattr(before, field) != getattr(after, field) for field in fields):
        raise Refus(f'lien source changeant pendant inspection : {path}')
    return {'chemin': str(path), 'readlink': link, 'index_mode': '120000',
            'index_blob': oid.decode('ascii'),
            'lstat': {field: getattr(after, field) for field in fields}}


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
    links = []
    for directory, dirs, files in os.walk(target, followlinks=False, onerror=walk_error):
        p = Path(directory)
        if any(name in dirs or name in files for name in ('.env', '.vault')):
            raise Refus('zone protégée présente : refus avant parcours ou status')
        # Une administration séparée peut être n'importe où sous les sources.
        if 'HEAD' in files and ('config' in files or 'commondir' in files or 'objects' in dirs):
            gitpath(p, '--git-common-dir', admin=True)
            admins.add(p)
            dirs[:] = []
            continue
        for name in list(dirs) + files:
            child = p / name
            if child.is_symlink() and name != '.git':
                try:
                    links.append(source_link(child))
                except (Refus, OSError) as exc:
                    refus.append(str(exc))
                if name in dirs:
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
            if any(name in dirs or name in files for name in ('.env', '.vault')):
                raise Refus('administration avec zone protégée : inspection refusée')
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
            state = worktree_state(c, sources)
            entries = state.pop('worktrees')
            descriptions.append({'administration': str(admin), 'commune': str(c),
                                 'worktrees': list(map(str, entries)), **state})
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
            'administrations': descriptions, 'archives': evidence, 'liens_sources': links,
            'refus': sorted(set(refus)),
            'limite': 'Quiescence à la charge du propriétaire ; aucune suppression autorisée après modification.'}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--depot', required=True, type=Path)
    parser.add_argument('--worktree', required=True, type=Path)
    parser.add_argument('--rapport', required=True, type=Path)
    parser.add_argument('--archive', nargs=2, action='append', default=[], metavar=('ADMINISTRATION', 'COPIE'))
    args = parser.parse_args()
    try:
        # Tous les arguments lexicaux d'abord : ne pas normaliser .vault/../x.
        paths = [args.depot, args.worktree, args.rapport]
        paths += [Path(p) for pair in args.archive for p in pair]
        for path in paths:
            protected(path)
        # Puis tous les chemins canoniques, avant le premier appel Git/ouverture.
        resolved = {path: safe_path(path, strict=path != args.rapport) for path in paths}
        target = resolved[args.worktree]
        repo = resolved[args.depot]
        report = resolved[args.rapport]
        if Path(os.path.abspath(args.worktree)) != target:
            raise ValueError('chemin de cible lié ou ambigu')
        if report.exists() or args.rapport.is_symlink():
            raise ValueError('rapport déjà présent : création exclusive exigée')
        report.parent.resolve(strict=True)
        if inside(report, target):
            raise ValueError('rapport situé dans la cible')
        archives = {}
        for source, copy in args.archive:
            key = resolved[Path(source)]
            if key in archives:
                raise ValueError('administration archive dupliquée')
            archives[key] = resolved[Path(copy)]
            if inside(report, key) or inside(report, archives[key]):
                raise ValueError('rapport situé dans une administration ou sa copie')
        if inside(report, gitpath(repo, '--git-common-dir')):
            raise ValueError('rapport situé dans une administration Git')
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
