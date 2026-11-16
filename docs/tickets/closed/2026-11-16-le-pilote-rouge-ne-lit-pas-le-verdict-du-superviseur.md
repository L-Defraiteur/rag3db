# Le pilote rouge ne lit pas le verdict du superviseur commun

- **État** : corrigé — lecteur bf507dd191, rouge c2b7639f5e puis huit témoins hermétiques verts ; aucun budget lourd consommé
- **Famille** : produit
- **Gravité** : preuve mal classée ; un rouge fonctionnel propre devient un défaut de montage apparent
- **Atteignable en service** : non, pilote de validation du lot mots vides Rust
- **Touche rag3weaver** : oui, preuve du pilote documentaire ; aucun changement runtime
- **Pour** : session codeparsers
- **Signalé par** : orchestration, revue RO ea0251cdf8 avant GO

## Cause et preuve à établir

pilot.py lit scope/verdict.json. Le vrai confiner_validation.py crée un
sous-dossier neuf via mkdtemp(prefix='validation-', dir=parent), puis y écrit
verdict.json et autorite.json. Le pilote doit lire un unique
scope/validation-*/verdict.json, refuser zéro ou plusieurs résultats, vérifier
que le reçu correspondant est terminé et correspond au chemin capturé par le
pilote, puis distinguer le code101/deux échecs attendus du reste.

## Travail et fin attendue

Avant correctif, témoin hermétique appelant le vrai lecteur avec la structure
commune ; aucun superviseur, Cargo, base ou service lancé. Puis lecture unique,
autorité terminée, rc101 au sous-processus et au verdict commun, deux tests
exactement/sans ignoré, erreur de supervision absente, restes vides et nettoyage
confirmé. Cas zéro/plusieurs verdicts et preuves incohérentes refusés.
Conserver cmd/rc/stdout/stderr et les rouges, publier nouveau SHA pour revue
avant GO1800. Ce ticket ne ferme ni le ticket std homonyme ni les mots vides.

## Preuves de correction

Le témoin du vrai lecteur a échoué sur c2b7639f5e : None !=101 (1échec,
0,001s), sorties conservées sous ~/.cache/codeparsers-notes/rust-red-reader/
before.{stdout,stderr,rc} sur le portable. Après bf507dd191 : 8tests verts.
La fixture finale utilise repertoire_preuves du commun pour créer le vrai
sous-dossier validation-*, sans le lancer ; huit verts à nouveau après cette
qualification de structure, after-common-layout.{stdout,stderr,rc}.
Pas de superviseur/autorité réelle, aucun Cargo/test Rust/service/base.
Le nouveau pilote exige un nouveau prepared.json et la revue/GO frais ;
les anciens manifestes restent préservés et ne doivent pas être réutilisés.
