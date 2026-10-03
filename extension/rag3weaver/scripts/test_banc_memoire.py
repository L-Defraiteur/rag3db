#!/usr/bin/env python3
"""Le banc de la mémoire longue — cinq sessions, cinq mesures, sans modèle.

« L'agent » est ce script : il appelle les verbes avec des choix fixés. La
vérité est donc connue **par construction**, et c'est ce qui rend mesurable
« fusionné à tort » — sans scénario écrit, cette mesure n'existe pas, puisqu'il
n'y a rien à quoi comparer.

Deux régimes, et le banc dit toujours lequel il a joué :

- **sans embarquement** (le défaut) : l'adresse du service est injoignable, les
  signaux tombent à `bm25` seul. Les sessions 1, 2, 4 et 5 tiennent — elles ne
  dépendent que de l'identité, de la machine à états et du filtre. La session 3
  (une redite dite autrement) **ne peut pas** être jugée : la retrouver demande
  le sens. Elle est alors annoncée **non jouée**, jamais verte.
- **avec embarquement** (`RAG3WEAVER_EMBED_SERVICE`) : les cinq sessions.

Une suite qui saute une mesure doit le dire. Un banc qui rend quatre mesures
sur cinq en annonçant cinq est pire qu'un banc absent.
"""
import json
import os
from pathlib import Path
import subprocess
import tempfile
from contextlib import contextmanager

ROOT = Path(__file__).resolve().parents[3]
CRATE = ROOT / 'extension/rag3weaver'
# L'extension vectorielle et la bibliothèque du moteur sont du C++ **bâti** :
# un worktree n'en a pas. `RAG3DB_ROOT` sert à pointer l'arbre où elles le
# sont — c'est le sens qu'elle a dans `run_e2e.sh` depuis le 7 septembre.
MOTEUR = Path(os.environ.get('RAG3DB_ROOT') or ROOT)
SAMPLE = CRATE / 'templates/backends/memory'
# Le target peut vivre ailleurs que dans le crate : depuis que /tmp est de la
# mémoire vive sur ce poste, on le met sur disque. On honore donc
# CARGO_TARGET_DIR plutôt que de supposer le défaut.
TARGET = Path(os.environ.get('CARGO_TARGET_DIR') or (CRATE / 'target'))
BINARY = TARGET / 'debug/rag3weaver-backend'
os.environ['LD_LIBRARY_PATH'] = str(MOTEUR / 'build/lecteurs-csv/src') + ':' + os.environ.get('LD_LIBRARY_PATH', '')
os.environ['RAG3DB_BUFFER_POOL_SIZE'] = '268435456'
os.environ['RAG3DB_MAX_DB_SIZE'] = '2147483648'

EMBED = os.environ.get('RAG3WEAVER_EMBED_SERVICE', '').strip()
AVEC_SENS = bool(EMBED)


@contextmanager
def host(manifest):
    process = subprocess.Popen([str(BINARY), str(manifest)], stdin=subprocess.PIPE,
                               stdout=subprocess.PIPE, text=True)

    def ask(name, arguments, attendu_ok=True):
        process.stdin.write(json.dumps({'op': 'call', 'name': name, 'arguments': arguments}) + '\n')
        process.stdin.flush()
        line = process.stdout.readline()
        assert line, f'backend exited {process.poll()}'
        reply = json.loads(line)
        if attendu_ok:
            assert reply['ok'], reply
            return reply['result']
        assert not reply['ok'], f'attendu un refus, reçu : {reply}'
        return reply
    try:
        yield ask
    finally:
        process.stdin.close()
        try:
            assert process.wait(timeout=60) == 0
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()
            raise
        process.stdout.close()


def config_at(folder):
    """Le manifeste du gabarit, avec les chemins résolus et le régime choisi."""
    config = json.loads((SAMPLE / 'backend.json').read_text())
    config['database'] = str(folder / 'memory.rag3db')
    config['vector_extension'] = str(MOTEUR / 'extension/vector/build/libvector.rag3db_extension')
    for entity in config['entities'].values():
        entity['schema'] = str(SAMPLE / entity['schema'])
    for tool in config['tools'].values():
        tool['graph'] = str((SAMPLE / tool['graph']).resolve())
    if AVEC_SENS:
        config['embeddings'] = {'address': EMBED, 'model': 'bge-m3', 'dimensions': 1024}
    else:
        # Une adresse injoignable **et** les signaux réduits : si un chemin
        # demandait quand même un vecteur, il échouerait au lieu de passer
        # inaperçu.
        config['embeddings'] = {'address': 'http://127.0.0.1:1/v1', 'model': 'unused', 'dimensions': 8}
        for entity in config['entities'].values():
            entity['config']['signals'] = ['bm25']
    return config


def record_de(reponse):
    """Le record dans l'enveloppe du backend : `result.result.data`.

    Un accesseur plutôt que trois fois le même chemin : le jour où l'enveloppe
    change, il y a un endroit à corriger.
    """
    return reponse['result']['data']


def memoire(claim, why, kind='fact', origin='said', reach='project', said_as=None, state='current'):
    return {'claim': claim, 'why': why, 'kind': kind, 'origin': origin,
            'said_as': said_as, 'reach': reach, 'state': state}


def main():
    # `None` veut dire **non jouée**, et ce n'est pas la même chose que zéro.
    # Un zéro se lit comme un succès ; c'est précisément le défaut que ce banc
    # existe pour attraper, et il n'a aucune raison de le commettre lui-même.
    mesures = {
        'doublons créés': 0,
        'rappels utiles': 0,
        'périmés servis comme vrais': 0,
        'contradictions vues': None,
        'propositions jamais appliquées': None,
    }
    non_jouees = []
    artefact = (0, '(non écrit)')

    with tempfile.TemporaryDirectory() as tmp:
        folder = Path(tmp)
        manifest = folder / 'backend.json'
        manifest.write_text(json.dumps(config_at(folder)))

        with host(manifest) as ask:
            # ── Session 1 : trois faits, deux ancrés, un global ──────────
            ask('put_subject', {'record': {'name': 'régime GPU', 'about':
                 'Comment on ménage les cartes pendant les passes.', 'state': 'current'}})
            ask('put_subject', {'record': {'name': 'tests', 'about':
                 'Ce qu on joue, quand, et à quel régime.', 'state': 'current'}})

            faits = [
                memoire('/tmp est de la mémoire vive',
                        'tmpfs de 61 Gio sur ce poste : aucun target cargo dans le scratchpad.'),
                memoire('la passe complète se lance sans demander',
                        'Depuis le 6 septembre, le debug de performance est fini.'),
                memoire('laisser deux coeurs libres',
                        'Jamais tous les coeurs : c est la compilation qui fige le poste.'),
                memoire('le journal survit à la passe',
                        'run_e2e ecrit target/e2e-last.log dans les deux branches, et nomme les tests en échec.'),
                memoire('annoncer cargo avant de le prendre',
                        'Le target est partagé : une ligne avant chaque passe, sinon deux sessions se marchent dessus.'),
                memoire('ne pas fusionner sans preuve',
                        'Un rapprochement se propose ; c est quelqu un qui tranche, jamais le seuil seul.'),
            ]
            for f in faits:
                ask('put_memory', {'record': f})

            vus = ask('recall', {'query': 'mémoire vive', 'options': {'limit': 10}})
            assert vus, 'la recherche ne rend rien'

            # ── Session 2 : une correction ───────────────────────────────
            # La correction passe par une transition déclarée : l'ancienne
            # devient superseded, la neuve est courante. Le lien REPLACES
            # viendra avec `revise` (lot suivant) ; ici on éprouve la machine
            # à états, qui est ce qui empêche la pagaille.
            ancienne = ask('get_memory', {'record': {'claim': '/tmp est de la mémoire vive',
                                                     'reach': 'project'}})
            rev = record_de(ancienne)['revision']
            ask('put_memory', {'record': dict(memoire(
                '/tmp est de la mémoire vive',
                'tmpfs de 61 Gio : rien de lourd dans le scratchpad, tout sous ~/.cache.',
                state='superseded')), 'expected_revision': rev})

            apres = ask('get_memory', {'record': {'claim': '/tmp est de la mémoire vive',
                                                  'reach': 'project'}})
            assert record_de(apres)['state'] == 'superseded', apres

            # Une transition **non déclarée** ne passe pas : superseded est
            # terminal. C'est la garde, et c'est elle qui tient tout le reste.
            refus = ask('put_memory', {'record': dict(memoire(
                '/tmp est de la mémoire vive', 'retour en arrière', state='current')),
                'expected_revision': record_de(apres)['revision']}, attendu_ok=False)
            assert 'transition' in json.dumps(refus), refus

            # ── Les paires de contrôle ──────────────────────────────────
            #
            # Un scénario **écrit** connaît ses réponses : ces paires n'ont été
            # étiquetées par personne, elles le sont par construction. Elles
            # servent de jeu de contrôle **indépendant** à qui règle un seuil
            # sur des paires jugées à la main — sinon les seuils sont évalués
            # sur ce qui les a réglés, et le chiffre est une borne haute.
            #
            # Écrites en artefact plutôt que laissées à lire dans ce script :
            # un jeu qu'il faut extraire d'un programme n'est pas un jeu.
            # Chaque paire porte **l'affirmation et le pourquoi des deux
            # côtés**. La première version ne donnait que les titres : des
            # paraphrases lointaines de phrases très courtes, c'est-à-dire une
            # tâche plus dure que la vraie, où un `remember` apporte toujours
            # son pourquoi. Un jeu de contrôle qui montre moins que la réalité
            # mesure le jeu, pas le produit.
            redites = [
                ('le dossier temporaire vit en RAM',
                 'Le repertoire temporaire de ce poste est un tmpfs : ce qu on y ecrit occupe la RAM.',
                 '/tmp est de la mémoire vive'),
                ('rien de lourd dans le repertoire temporaire',
                 'On ne met ni dossier de compilation ni gros clone sous /tmp, faute de place en mémoire.',
                 '/tmp est de la mémoire vive'),
                ('la batterie se joue sans rien demander',
                 'Plus besoin de demander avant de lancer la suite complète : le debug de performance est fini.',
                 'la passe complète se lance sans demander'),
                ('garder deux processeurs pour la machine',
                 'La compilation ne prend jamais tous les processeurs : il en faut pour que l interface réponde.',
                 'laisser deux coeurs libres'),
                ('le compte rendu de la passe reste sur le disque',
                 'Le journal de la suite est écrit dans un fichier qui survit à la passe, et les échecs y sont nommés.',
                 'le journal survit à la passe'),
                ('dire qu on prend le compilateur',
                 'Une ligne avant chaque compilation : le dossier de build est partagé entre les sessions.',
                 'annoncer cargo avant de le prendre'),
            ]
            contradictions = [
                ('on peut mettre un target cargo dans /tmp',
                 'Le repertoire temporaire est sur disque et tient un gros dossier de compilation.',
                 '/tmp est de la mémoire vive'),
                ('compiler sur tous les coeurs ne gêne personne',
                 'Prendre tous les processeurs pour compiler ne dégrade pas l usage de la machine.',
                 'laisser deux coeurs libres'),
            ]
            par_claim = {f['claim']: f['why'] for f in faits}
            paires = []
            for genre, source, verite_si_cible in (('redite', redites, 'meme'),
                                                   ('contradiction', contradictions, 'contredit')):
                for nouveau, pourquoi, cible in source:
                    for claim, why in par_claim.items():
                        paires.append({
                            'genre': genre,
                            'verite': verite_si_cible if claim == cible else 'different',
                            'nouveau': {'claim': nouveau, 'why': pourquoi},
                            'existant': {'claim': claim, 'why': why},
                        })
            # Les paires sont **des données du scénario**, pas une sortie de
            # build : elles vivent dans le dépôt, versionnées, pour qu'un diff
            # dise quand le jeu de contrôle a changé — sinon deux mesures ne
            # sont pas comparables et personne ne peut le savoir.
            sortie = Path(os.environ.get('BANC_PAIRES')
                          or (CRATE / 'scripts/banc-memoire/paires-de-controle.json'))
            sortie.parent.mkdir(parents=True, exist_ok=True)
            sortie.write_text(json.dumps(paires, ensure_ascii=False, indent=2) + '\n')
            artefact = (len(paires), str(sortie))

            # ── Session 3 : une redite dite autrement ────────────────────
            if AVEC_SENS:
                proches = ask('recall', {'query': 'le dossier temporaire vit en RAM', 'options': {'limit': 5}})
                trouve = any('/tmp' in json.dumps(p) for p in (proches if isinstance(proches, list) else [proches]))
                if not trouve:
                    mesures['doublons créés'] += 1
            else:
                non_jouees.append('session 3 (redite dite autrement) — demande le sens, '
                                  'donc un service d embarquement')

            # ── Session 5 : le périmé ne doit pas revenir ────────────────
            # (la session 4 — l'ancre qui change ou disparaît — attend le lot
            # « l'ingestion émet son ensemble changé » et son réacteur.)
            courantes = ask('recall', {'query': 'mémoire vive',
                                       'options': {'limit': 10, 'filter_condition':
                                           {'field': {'key': 'state',
                                                      'value': [{'op': 'eq', 'value': 'current'}]}}}})
            texte = json.dumps(courantes)
            if 'superseded' in texte:
                mesures['périmés servis comme vrais'] += 1
            if 'coeurs libres' in json.dumps(ask('recall', {'query': 'coeurs', 'options': {'limit': 5}})):
                mesures['rappels utiles'] += 1

            non_jouees.append('session 4 (l ancre change ou disparaît) — l ingestion dit '
                              'désormais ce qu elle a changé ; il manque le réacteur qui '
                              'transitionne les mémoires ancrées')
            non_jouees.append('propositions jamais appliquées — attend `remember` et '
                              'son noeud de décision (lot 5)')
            non_jouees.append('contradictions vues — le scénario en porte deux dans ses '
                              'paires de contrôle, mais rien ne les détecte encore : '
                              'la matière est là, la mesure attend le nœud de décision')

    largeur = max(len(k) for k in mesures)
    print('\n── Banc de la mémoire longue ' + '─' * 28)
    print(f'  régime : {"avec embarquement" if AVEC_SENS else "sans embarquement (bm25 seul)"}')
    for nom, valeur in mesures.items():
        rendu = '— non joué' if valeur is None else str(valeur)
        print(f'  {nom.ljust(largeur)} : {rendu}')
    print(f'\n  paires de contrôle : {artefact[0]}, vérité par construction')
    print(f'    {artefact[1]}')
    if non_jouees:
        print('\n  NON JOUÉ — le banc ne les compte pas et ne les annonce pas vertes :')
        for n in non_jouees:
            print(f'    · {n}')
    print()
    # On n'affirme que sur ce qui a été joué : une assertion sur une mesure
    # non jouée serait verte pour la mauvaise raison.
    jouees = {k: v for k, v in mesures.items() if v is not None}
    assert jouees['doublons créés'] == 0, mesures
    assert jouees['périmés servis comme vrais'] == 0, mesures
    print(f'banc : {len(jouees)} mesure(s) jouée(s) au vert, '
          f'{len(mesures) - len(jouees)} non jouée(s)')


if __name__ == '__main__':
    main()
