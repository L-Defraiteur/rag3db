# Les commandes et options du terminal en anglais, avec alias

- **État** : ouvert. Règle de Lucie du 16 nov. 2026 (« stop au code en français, ou aux /commandes en français ») ; rien n'est encore fait dans le terminal.
- **Famille** : produit
- **Gravité** : forme. Les commandes et options françaises restent, mais toute nouveauté est en anglais, et l'existant prend un alias quand on y touche.
- **Atteignable en service** : oui, à chaque lancement du terminal et à chaque `/commande`.
- **Touche rag3weaver** : oui (le crate `extension/rag3weaver/tty`).

## Ce qu'il faut faire

Ajouter un nom anglais à chaque `/commande` et option française, en gardant l'ancien nom, qui marche toujours et que `/aide` dit « ancien nom ».

| Français (gardé) | Anglais (ajouté) |
|---|---|
| `/aide` | `/help` |
| `/quitter` | `/quit` |
| `/arreter` | `/stop` |
| `/theme` | `/theme` (déjà) |
| `/tache` | `/task` |
| `/depot` | `/repo` |
| `/journal` | `/log` |
| `/effacer`, `/effacer historique` | `/clear`, `/clear history` |
| `/context` | `/context` (déjà) |
| `/etapes` | `/steps` |
| `/model` | `/model` (déjà) |
| `/nom` | `/name` |
| `--depot` | `--repo` |
| `--tache` | `--task` |
| `--demande` | `--request` |
| `--journal` | `--log` |
| `--etapes` | `--steps` |
| `--depuis` | `--since` |
| `--adresse` | `--address` |

- `src/commandes.rs` : `lire` accepte les deux noms ; `COMMANDES` montre le nom anglais en premier dans `/aide`, avec « (ancien : /nom) » ; la complétion par Tab propose les noms anglais.
- `src/main.rs` : les options anglaises, et l'aide (`AIDE`) avec les noms anglais ; les anciens sont lus sans avertissement pour l'instant (pas de bruit pour Lucie), et le README dit qu'ils sont anciens.
- `extension/rag3weaver/tty/README.md` : les noms anglais d'abord.
- Ne rien retirer : les scripts de Lucie et des sessions lancent `--depot`, `--tache`, `--demande`, `--journal` (voir `docs/orchestration/16-11-2026/01-essais-empiriques-algareth.md`).
- Le texte affiché reste en français.

## La preuve

- Test de `commandes.rs`, rouge d'abord : chaque nom anglais rend la même `Commande` que le français ; Tab sur `/st` propose `/steps` et `/stop`.
- Test de lecture des options de `main.rs` : extraire la lecture des arguments dans une fonction testable si elle ne l'est pas (`lire_options`), puis un test qui lit les deux jeux de noms.
- Passe courte : crate, `check_features`, harnais entier (les séances lancent le terminal avec les anciens noms : elles doivent rester vertes).

## Taille et blocage

3 heures. Rien ne le bloque.


## Reprise du 16 novembre — témoins préparés

Porteur tty ; branche `cx-tty-english-aliases`, worktree
`/home/lucied/git_workspaces/cx-tty-english-aliases`, base `927941a67a`.
Réservation : `tty/src/commandes.rs`, `tty/src/main.rs`, `tty/README.md` et tests
locaux ; aucun changement d'hôte ou de thème, aucun besoin de `tui.rs` identifié.
Deux témoins ajoutés pour équivalence des commandes et complétion anglaise.
NON JOUÉS : la compilation et le harnais attendent FINI de la fenêtre optimiseur.
Aucun rouge observé encore, aucun correctif appliqué. Prochain pas : jouer les
témoins, conserver le rouge puis corriger ; parser CLI testable et compatibilité
`--task` silencieuse à couvrir. Aucun ancien créneau ne se réactive.

### Relecture mémoire et recette ciblée avant correctif

Branche fraîche `cx-tty-english-aliases-2`, rebasée sur master `f83c6c4dd6`.
`--task` déjà accepté doit devenir silencieux, aussi avec `--backend` ;
conserver options françaises/backend et dernière valeur rencontrée.
Extraire le parser avec arguments injectés et booléen steps dans Options :
ne pas muter l'environnement dans les tests parallèles. Couvrir tables EN=FR,
valeurs absentes/répétées/mixtes, aide et README anglais prioritaires, anciens
noms explicites. Aucun correctif avant assertions rouges réellement observées.

Recette publiée `docs/tty/16-11-2026/red-english-aliases.sh` : uniquement les
2 témoins `english_`, crate lib sans vue, offline/locked/j4, target privé.
Budget proposé 600 s global, plafond32G, sous confiner_validation/poste lourd ;
aucun SDK natif, modèle, hôte, fixture réseau ni base de Lucie nécessaire.
Le script conserve commande/rc/sortie avant jugement et distingue un refus de
préparation du rouge d'assertions. À jouer au GO de l'orchestration seulement.

### Rouge observé avant correctif

Source exacte `4cc0b98a8c93093ba7dad63edd74287df7d798e9` : worktree propre,
target et dossier de preuve absents avant entrée. Le pilote borné autorisé a
compilé le crate depuis ce worktree (2,83 s), puis joué **2 tests, 0 passés,
2 échoués, 0 ignoré** : `/help` donne `Inconnue`, `/st` ne se complète pas.
Cargo rc101 ; pilote rc1. Preuves conservées hors worktree :
`~/.cache/rag3weaver-build/cx-tty-english-aliases-red-v1/` (commande,source,rc,log,verdict).
Common `~/.cache/rag3weaver-build/validations/validation-g4dqpr6q/` :
verdict ROUGE attendu, erreur null, restes [], nettoye true. Aucun service/SDK.
Ce résultat remplace le NON JOUÉ des deux témoins ci-dessus seulement.

### Préparation GO2 — provenance du target conservée, garde à résoudre

Le target unique reste `~/.cache/rag3weaver-build/target-cx-tty-english-aliases-2`.
Il était absent avant le rouge ; source et autorité terminée ci-dessus. ELF rouge
`debug/deps/rag3weaver_tty-bf2e988222854013`, SHA256
`b2b6101a8face2ebda84400a31ad653444e576167e9198410dc6a64ad83d3015`.
Le pilote rouge avait appelé Cargo directement : aucun `.arbre-du-target` n'a
été posé. La garde courante refuse explicitement un target non vide sans ce
marqueur. Pas de marqueur fabriqué ni de nouveau target créé : demander à
l'orchestration la procédure de provenance qualifiée avant GO2 ; ne pas éluder
ce refus par un appel Cargo direct. Le rouge d'assertions observé reste conservé.

Features vérifiées dans Cargo.toml : lib minimale sans vue ; le binaire exige
`vue`, dont seules ratatui/unicode-width/unicode-segmentation sont optionnelles.
Aucun SDK natif, modèle ou moteur n'est une dépendance de ce crate. Chaîne
proposée après résolution de la garde : même target, `cargo test --offline
--locked -j4 --no-default-features --lib english_` (2 verts attendus), puis
`cargo test --offline --locked -j4 --bin rag3weaver-tty english_options_tests`
(vue par défaut, rouge CLI attendu), Common unique600s/32G ; conserver les deux
rc/logs avant jugement, aucun retry, aucun nouveau cache. Aucun de ces essais
n'a été lancé. Correctif CLI et README toujours à faire après rouge CLI.

### Retrait préparé puis arrêté sur refus réel de garde

Archive tar fidèle du target, ELF rouge, preuves et reçu :
`~/.cache/rag3weaver-build/archives/cx-tty-aliases-red-20261116/target-and-proof.tar`,
copiée sur luciepc au même chemin relatif. SHA256 comparé identique aux deux
postes : `56aca2b2488b99a302f0e806724bf58a2677805a9413f8ca9152ac2caf9d09f3`.
Garde courante exécutée depuis le worktree propriétaire, cible target seul :
rc2 ROUGE, « inspection Git impossible » sur ce cache sans .git.
Rapport hors target `archives/cx-tty-aliases-red-20261116/garde-target-retrait.json`.
La garde exige un worktree Git enregistré et ne juge pas un target indépendant.
STOP transmis à l'orchestration ; aucun retrait, marqueur, target neuf ou Cargo.
Le worktree source reste intact. Attente de procédure applicable au target seul,
sans assimiler un verdict sur le worktree à une autorisation de retrait du cache.

### Alternative réversible : arrêt avant renommage

L'orchestration a autorisé un relogement entier par rename même système de
fichiers, sans réutilisation d'artefacts. Précontrôles UID/dev/type/répertoire
source, destination absente et autorité terminée passent. Le scan frais de tous
les processus du même UID (exe/cwd/FD) ne trouve aucun lien vers le target,
mais rencontre des accès refusés sur les PID 5435,5438,6208,25298. L'absence de
writer n'est donc pas établie pour eux : arrêt conservateur avant rename.
Source encore présente, destination `retired-target` absente, aucune mutation
du target ni Cargo. Archives aux deux postes et worktree toujours conservés.
Prochain pas : qualifier ces inconnus avec l'orchestration avant relogement ;
ne pas transformer « aucun lien observé » en preuve de quiescence complète.

### Relogement réversible effectué, GO2 distinct à venir

Implémentation `relocate_target.py` publiée avant action à `1c08020816` ;
3 tests filesystem factices passés, dont refus atomique d'une destination
existante par renameat2 RENAME_NOREPLACE. Exécution réelle ensuite réussie :
`archives/cx-tty-aliases-red-20261116/relocation-atomic.json`, état
RELOCATED_IDENTICAL, snapshots complets identiques avant/après, archives tar
recontrôlées aux deux postes, autorité terminée et scan frais conservés.
Target ancien intégral sous `retired-target`, mêmes inodes/modes/SHA ; chemin
T de session désormais absent. UNKNOWN baseline non attribués gardés dans le
rapport, pas assimilés à zéro processus. Aucun Cargo ni target actif créé.

Prochain pilote publié `check-english-aliases-v2.sh` : création exclusive du
même T absent, vraie garde initiale sur le crate propriétaire puis deux essais
bornés au prochain GO600 : commandes lib sans vue (2 verts attendus), options
binaire vue par défaut (1 rouge attendu). Journaux/rc conservés séparément,
Common unique32G/j4, aucun SDK/hôte/modèle. NON JOUÉ.

### GO2 terminé et correctif CLI publié

Sur source11a59d3be7 : commandes 2/2 vertes, CLI 0/1 rouge attendu
`option inconnue : --repo`, zéro ignoré. Preuves privées
`cx-tty-english-aliases-go2-v1`, Common `validation-qtgpttem` terminé,
code1/erreur null/restes[]/nettoye true ; HEAD et worktree propres avant/après.
FixCLI+README publié `3be38590ac` après clôture du tour Embarquement.
Nouvelle branche `cx-tty-english-aliases-4` rebasée sur master
`bef0b27cb33b9143dabee019fccdf7453da553a3` ; range-diff des11commits
équivalent, conservé horsW sous `cx-tty-aliases-union-review/range-diff.txt`.
Aucun changement hôte/thème/tui. Parser testable avec arguments injectés,
steps stocké en booléen puis appliqué seulement au lancement, --task silencieux,
FR/backend préservés. CLI/tableau et aide/README restent NON QUALIFIÉS après fix.

Proposition suivante : Common600s/32G/j4, même target connu gardé,
`check-english-aliases-green.sh`, tous tests crate vue puis minimal offline/locked,
états XDG privés (cache inchangé, aucun SDK). Harnais et check_features produit
restent à préparer et jouer séparément ; ne pas annoncer le ticket clos sur le
seul vert crate. Aucun nouveau Cargo lancé à ce stade.

### Suite F8 et harnais — préparation sur master frais

Crate source0c4ab173d4 : vue113lib+1CLI/minimal22 passés, tous0échec/0ignoré ;
Common `validation-6tdyny5z` VERT0/erreur null/restes[]/nettoye true.
Nouvelle union `cx-tty-english-aliases-5` sur master
`57fbd9aa56f4b7caeacb891388f3f10cbbb39d71` : depuis basebef, entrants
procédure/outil de retour de snapshots seulement, aucun Rust/tty/SDK ajouté.
Diff de périmètre audité en lecture seule, aucun nouveau Cargo.

Pilote proposé `check-english-aliases-full.sh` : F8 canonique8/8 puis bâti
binaire et harnais tmux-local-ouvert entier avec anciens noms. Même target
session, gardes réelles de changement propriétaire workspace→crate ; aucun
second cache d'artefacts. SDK59ac épinglé avec magasin explicite et SHA so/vector,
cache/états XDG privés frères, aucun modèle ou hôte partagé. Port17990 libre
observé en préparation ; il doit encore l'être à l'entrée, sinon arrêt.
Le harnais démarre uniquement son faux hôte privé et ses sessions tmux privées.
Son état historique en dur `$HOME/.local/state` serait inadapté : shim publié
`run_private_harness.py` substitue seulement ETAT_TTY au chemin privé et conserve
source générée+empreintes, sans changer HOME, scénarios, commandes ou résultats.

Coût proposé : Common unique7200s/32G/j4, un budget non réinitialisé pour F8,
bâti et harnais ; CHECK_FEATURES_TIMEOUT1200 par jeu à l'intérieur du même
budget. Target plafond32G. Aucun lancement avant GO distinct ; premier refus
arrête sans retry. Prérequis à réserver : créneau lourd CPU et port privé17990 ;
tmux présent sur le portable, SDK59ac et archives antérieures présents.
Pas de service partagé, aucune ouverture des bases de Lucie, aucun privilège.
F8 et harnais restent NON JOUÉS sur le correctif jusqu'au verdict réel.
