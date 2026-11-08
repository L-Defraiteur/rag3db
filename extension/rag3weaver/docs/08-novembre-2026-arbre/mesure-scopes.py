# Combien de scopes une vue apporte vraiment : pour chaque branche (contre sa base, le merge-base),
# chaque fichier de code changé est découpé (codeparsers, voir_json) à la base et à la tête ; les
# scopes se comparent par (nom, type, rang) et par contenu.
import subprocess, json, sys, os, tempfile, hashlib, collections
DEPOT = os.path.expanduser('~/git_workspaces/rag3db')
VJ = os.path.expanduser('~/.cache/rag3weaver-build/target-plan-local/release/examples/voir_json')
EXT = ('.rs', '.cpp', '.h', '.hpp', '.ts', '.tsx', '.py', '.js')
def git(*a):
    return subprocess.run(['git', '-C', DEPOT, *a], capture_output=True, text=True).stdout
def scopes(rev, chemin, dossier):
    blob = subprocess.run(['git', '-C', DEPOT, 'show', f'{rev}:{chemin}'], capture_output=True)
    if blob.returncode != 0:
        return None
    f = os.path.join(dossier, os.path.basename(chemin))
    open(f, 'wb').write(blob.stdout)
    r = subprocess.run([VJ, f], capture_output=True, text=True)
    try:
        sc = json.loads(r.stdout).get('scopes', [])
    except Exception:
        return []
    vus = collections.Counter(); out = {}
    for s in sc:
        if s.get('type') == 'Lambda': continue
        k = (s.get('name'), s.get('type'))
        vus[k] += 1
        out[(k, vus[k])] = (hashlib.blake2b((s.get('content') or '').encode(), digest_size=8).hexdigest(), s.get('scope_start_line'))
    return out
tot = collections.Counter(); types = collections.Counter()
for b in open(sys.argv[1]).read().split():
    mb = git('merge-base', 'origin/master', b).strip()
    fichiers = [l for l in git('diff', '--name-only', mb, b).split() if l.endswith(EXT)]
    c = collections.Counter(fichiers=len(fichiers))
    with tempfile.TemporaryDirectory() as d:
        for f in fichiers:
            avant = scopes(mb, f, d) or {}
            apres = scopes(b, f, d)
            if apres is None:
                c['retires'] += len(avant); continue
            c['scopes_tete'] += len(apres)
            for k, (h, ligne) in apres.items():
                if k not in avant: c['neufs'] += 1
                elif avant[k][0] != h:
                    c['modifies'] += 1; types[k[0][1]] += 1
                elif avant[k][1] != ligne: c['deplaces'] += 1
                else: c['inchanges'] += 1
            c['retires'] += sum(1 for k in avant if k not in apres)
    tot.update(c)
    print(f"{b:40s} fichiers={c['fichiers']:3d} scopes_tete={c['scopes_tete']:5d} neufs={c['neufs']:4d} modifies={c['modifies']:4d} deplaces={c['deplaces']:4d} inchanges={c['inchanges']:5d} retires={c['retires']:3d}")
print('TOTAL', dict(tot)); print('MODIFIÉS PAR TYPE', types.most_common(12))
