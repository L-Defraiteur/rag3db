#!/usr/bin/env python3
"""Validation Linux : poste réel, budget global, descendants bornés et bus privé."""
import argparse
import ctypes
import hashlib
import json
import os
from pathlib import Path
import secrets
import shutil
import signal
import subprocess
import sys
import tempfile
import time

MAX_MEMOIRE = 32 * 1024**3
RECU = 'RAG3WEAVER_VALIDATION_RECU'
JETON = 'RAG3WEAVER_VALIDATION_JETON'


class Refus(RuntimeError):
    pass


def processus():
    result = {}
    for p in Path('/proc').iterdir():
        if not p.name.isdigit():
            continue
        try:
            if p.stat().st_uid != os.getuid():
                continue
            st = (p / 'stat').read_text().rsplit(') ', 1)[1].split()
            if st[0] == 'Z':
                continue
            result[int(p.name)] = dict(parent=int(st[1]), session=int(st[3]), debut=st[19],
                                       cgroup=(p / 'cgroup').read_text().split('0::', 1)[1].strip())
        except (OSError, IndexError):
            continue
    return result


def sous(chemin, racine):
    return chemin == racine or chemin.startswith(racine.rstrip('/') + '/')


def plafond(cgroup, racine=Path('/sys/fs/cgroup')):
    """memory.max est hiérarchique : relever le direct ET le minimum hérité."""
    if not cgroup.startswith('/') or '..' in Path(cgroup).parts:
        raise Refus('chemin cgroup ambigu')
    p = racine / cgroup.lstrip('/')
    limites = []
    direct = None
    while p != racine:
        valeur = (p / 'memory.max').read_text().strip()
        if direct is None:
            direct = valeur
        if valeur != 'max':
            limites.append(int(valeur))
        p = p.parent
    effectif = min(limites) if limites else None
    if effectif is None or effectif <= 0 or effectif > MAX_MEMOIRE:
        raise Refus('aucun plafond effectif <= 32 Gio')
    return dict(direct=direct, effectif=effectif)


def ancetres(pid, table):
    vus = set()
    while pid in table and pid not in vus:
        vus.add(pid)
        yield pid
        pid = table[pid]['parent']


def poste_tenu(table):
    verrous = Path(os.environ.get('POSTE_LOCK_DIR', str(Path.home() / '.cache/rag3weaver-build')))
    for pid in ancetres(os.getpid(), table):
        try:
            fiche = (verrous / 'fiches' / str(pid)).read_text()
            args = (Path('/proc') / str(pid) / 'cmdline').read_bytes().split(b'\0')
            if fiche.startswith(('tient lourd ', 'tient mesure ')) and any(Path(os.fsdecode(a)).name == 'poste' for a in args if a):
                return pid
        except OSError:
            pass
    return None


def autorite(table):
    """Un reçu n'est valable que chez un descendant de son superviseur vivant."""
    try:
        p = Path(os.environ[RECU])
        if p.is_symlink() or p.stat().st_uid != os.getuid() or p.stat().st_mode & 0o077:
            raise Refus('reçu non privé')
        r = json.loads(p.read_text())
        pid = r['pid']
        if (r['jeton'] != os.environ[JETON] or r['etat'] != 'actif'
                or pid not in set(ancetres(os.getpid(), table))
                or table[pid]['debut'] != r['debut'] or table[pid]['cgroup'] != r['cgroup'] or not poste_tenu(table)
                or not sous(table[os.getpid()]['cgroup'], r['cgroup'])):
            raise Refus('reçu sans superviseur/poste réel dans notre ascendance')
        plafond(table[os.getpid()]['cgroup'])
        if time.monotonic() >= r['echeance']:
            raise Refus('budget du superviseur expiré')
        return r
    except (KeyError, OSError, ValueError) as e:
        raise Refus('validation non supervisée') from e


def configuration_bus(socket):
    # Le chemin est créé par mkdtemp ; aucun XML fourni par un appelant.
    from xml.sax.saxutils import escape
    return ('<busconfig><type>session</type><listen>unix:dir=' + escape(str(socket)) + '</listen>'
            '<auth>EXTERNAL</auth><policy context="default"><allow user="' + str(os.getuid()) + '"/>'
            '<allow own="*"/><allow send_destination="*"/><allow receive_sender="*"/>'
            '</policy></busconfig>\n')


def repertoire_preuves(valeur):
    parent = Path(valeur).expanduser() if valeur else Path.home() / '.cache/rag3weaver-build/validations'
    if parent.is_symlink():
        raise Refus('racine de preuves liée')
    parent.mkdir(parents=True, exist_ok=True)
    return Path(tempfile.mkdtemp(prefix='validation-', dir=parent))


class Descendants:
    def __init__(self, racine, cgroup):
        self.racine, self.cgroup = racine, cgroup
        self.possedes = {}
        self.observes = {}

    def relever(self, table):
        # subreaper : même un petit-fils détaché rejoint le superviseur à la mort de son parent.
        change = True
        while change:
            change = False
            for pid, r in table.items():
                if pid not in self.possedes and (pid == self.racine or r['parent'] in self.possedes or r['parent'] == os.getpid()):
                    self.possedes[pid] = r['debut']
                    change = True
        erreurs = []
        for pid, debut in self.possedes.items():
            r = table.get(pid)
            if not r or r['debut'] != debut:
                continue
            row = dict(pid=pid, **r)
            try:
                row['memoire'] = plafond(r['cgroup'])
                if not sous(r['cgroup'], self.cgroup):
                    raise Refus('descendant hors du cgroup de validation')
            except (Refus, OSError, ValueError) as e:
                row['refus'] = str(e)
                erreurs.append(row)
            self.observes[(pid, debut, r['cgroup'])] = row
        if erreurs:
            raise Refus('confinement enfant : ' + json.dumps(erreurs))

    def vivants(self):
        t = processus()
        return [pid for pid, debut in self.possedes.items() if pid in t and t[pid]['debut'] == debut]

    def signaler(self, sig):
        for pid in self.vivants():
            try:
                fd = os.pidfd_open(pid)
                try:
                    t = processus()
                    if pid in t and t[pid]['debut'] == self.possedes[pid]:
                        signal.pidfd_send_signal(fd, sig)
                finally:
                    os.close(fd)
            except ProcessLookupError:
                pass

    def arreter(self):
        # Toujours inventorier les nouveaux orphelins avant les signaux.
        try:
            self.relever(processus())
        except Refus:
            pass
        for sig in (signal.SIGTERM, signal.SIGKILL):
            self.signaler(sig)
            fin = time.monotonic() + .7
            while self.vivants() and time.monotonic() < fin:
                try:
                    self.relever(processus())
                except Refus:
                    pass
                if sig == signal.SIGKILL:
                    self.signaler(sig)
                time.sleep(.025)
        return self.vivants()


def superviser(command, delai, preuves):
    commence = time.monotonic()
    table = processus()
    if not poste_tenu(table):
        raise Refus('aucun poste réellement tenu')
    cg = table[os.getpid()]['cgroup']
    memoire = plafond(cg)
    if not hasattr(os, 'pidfd_open') or not hasattr(signal, 'pidfd_send_signal'):
        raise Refus('pidfd Linux requis')
    outils = {n: shutil.which(n) for n in ('dbus-run-session', 'dbus-daemon')}
    if not all(outils.values()):
        raise Refus('dbus-run-session et dbus-daemon requis, aucun repli')
    # Préserver les orphelins attribués : ne jamais rechercher/tuer par nom.
    libc = ctypes.CDLL(None, use_errno=True)
    if libc.prctl(36, 1, 0, 0, 0) != 0:  # PR_SET_CHILD_SUBREAPER
        raise Refus('subreaper indisponible')
    evidence = repertoire_preuves(preuves)
    prive = Path(tempfile.mkdtemp(prefix='r3v-', dir='/tmp'))
    try:
        socket = prive / 'bus'
        socket.mkdir(mode=0o700)
        config = prive / 'bus.xml'
        config.write_text(configuration_bus(socket))
        shutil.copy2(config, evidence / 'bus.xml')
        echeance = commence + delai
        recu = dict(pid=os.getpid(), debut=table[os.getpid()]['debut'], cgroup=cg,
                    jeton=secrets.token_hex(24), etat='actif', echeance=echeance)
        recu_path = evidence / 'autorite.json'
        recu_path.touch(mode=0o600)
        recu_path.write_text(json.dumps(recu))
        env = dict(os.environ, **{RECU: str(recu_path), JETON: recu['jeton']},
                   RAG3WEAVER_VERROU_TENU='validation supervisée', RAG3WEAVER_BUILD_MEMORY_HIGH='0')
        argv = [outils['dbus-run-session'], '--config-file=' + str(config), '--', *command]
        (evidence / 'recette.json').write_text(json.dumps(dict(argv=argv, cgroup=cg, memoire=memoire,
             delai=delai, outils={n: dict(path=p, sha256=hashlib.sha256(Path(p).read_bytes()).hexdigest()) for n, p in outils.items()}), indent=2))
    except BaseException:
        shutil.rmtree(prive)
        raise
    enfant = None
    groupe = None
    erreur = None
    code = 1
    ancien = {}
    def interrompre(sig, frame):
        raise Refus('signal reçu : ' + str(sig))
    for sig in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP):
        ancien[sig] = signal.signal(sig, interrompre)
    try:
        if time.monotonic() >= echeance - 2:
            raise Refus('budget épuisé pendant le montage, commande non lancée')
        enfant = subprocess.Popen(argv, env=env, start_new_session=True)
        groupe = Descendants(enfant.pid, cg)
        while True:
            groupe.relever(processus())
            if enfant.poll() is not None:
                code = enfant.returncode
                if groupe.vivants():
                    raise Refus('descendants laissés après la commande')
                break
            # Réserver le nettoyage dans le délai global, pas après son échéance.
            if time.monotonic() >= echeance - 2:
                raise Refus('échéance de validation')
            time.sleep(.025)
    except (Refus, OSError) as e:
        erreur = str(e)
    finally:
        restes = groupe.arreter() if groupe else []
        if enfant is not None:
            try:
                enfant.wait(timeout=.2)
            except subprocess.TimeoutExpired:
                restes.append(enfant.pid)
        while True:
            try:
                if os.waitpid(-1, os.WNOHANG)[0] == 0:
                    break
            except ChildProcessError:
                break
        recu['etat'] = 'termine'
        recu_path.write_text(json.dumps(recu))
        (evidence / 'enfants.json').write_text(json.dumps(list(groupe.observes.values()) if groupe else [], indent=2))
        # Une zone temporaire attribuée reste préservée si son détenteur subsiste.
        if not restes:
            shutil.rmtree(prive)
        verdict = dict(verdict='VERT' if code == 0 and not erreur and not restes else 'ROUGE',
                       code=code, erreur=erreur, restes=restes, prive=str(prive), nettoye=not prive.exists())
        (evidence / 'verdict.json').write_text(json.dumps(verdict, indent=2))
        for sig, handler in ancien.items():
            signal.signal(sig, handler)
    print('validation : ' + verdict['verdict'] + ' ; preuves : ' + str(evidence), file=sys.stderr)
    return 0 if verdict['verdict'] == 'VERT' else 1


def main():
    p = argparse.ArgumentParser(description=__doc__)
    actions = p.add_subparsers(dest='action', required=True)
    actions.add_parser('verifier')
    lancer = actions.add_parser('lancer')
    lancer.add_argument('--delai', type=int, required=True)
    lancer.add_argument('--mode', choices=['lourd', 'mesure'], default='lourd')
    lancer.add_argument('--preuves')
    lancer.add_argument('commande', nargs=argparse.REMAINDER)
    a = p.parse_args()
    try:
        table = processus()
        if a.action == 'verifier':
            autorite(table)
            return 0
        command = a.commande[1:] if a.commande[:1] == ['--'] else a.commande
        if not command or a.delai is None or a.delai < 3:
            raise Refus('commande et délai global >= 3 secondes requis')
        if RECU in os.environ or JETON in os.environ:
            autorite(table)  # Un reçu périmé/forgé n'autorise pas de nouveau superviseur.
            raise Refus('validation déjà supervisée : composer la commande dans cette portée')
        if not poste_tenu(table):
            poste = Path(__file__).with_name('poste')
            env = dict(os.environ)
            env.setdefault('POSTE_MEM_MAX', '32G')
            env.pop('RAG3WEAVER_VERROU_TENU', None)
            os.execve(str(poste), [str(poste), a.mode, sys.executable, str(Path(__file__).resolve()), *sys.argv[1:]], env)
        return superviser(command, a.delai, a.preuves)
    except (Refus, OSError, ValueError) as e:
        print('ROUGE validation : ' + str(e), file=sys.stderr)
        return 2


if __name__ == '__main__':
    sys.exit(main())
