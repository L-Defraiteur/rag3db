# Les commandes et options du terminal en anglais, avec alias

- **État** : ouvert — alias implémentés, crate qualifié sur `0c4ab173d4` (vue113+1CLI, minimal22 ; zéro échec/ignoré), F8 canonique 8/8 sur `6d886375ea` ; bâti tty arrêté avant compilation sur dépendance offline absente du lock séparé, harnais NON JOUÉ. Les anciens HOLD de préparation restent historiques.
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

### Révision RO du pilote F8/harnais avant GO

Union neuve `cx-tty-english-aliases-6` sur `b0a845bb4877328eada37951c174e3d208986630`.
Entrant Rust : uniquement cfg(test) codex_threads, préparation des exécutables
factices dans un sous-processus borné ; nouveau test de caractères littéraux,
aucun contrat terminal/runtime modifié. F8 vérifiera sa compilation ; aucun
résultat d'exécution de ces tests hôte n'est revendiqué par le harnais tty.

Préflight après vrai poste/Common AVANT garde : reçu précédent6tdyny terminé,
starttime/PID, scan FD/exe/cwd avec inconnus conservés, propriétaire UID/dev/ino,
ELF précédents et archive rouge réhashés. Réutilise scan publié du relogement
et autorité Common réelle, sans permission ni mutation des processus tiers.
L'absence de liens attribués n'efface pas les UNKNOWN non attribués.

SDK : résolution/hash sans exporter avant F8 ; seul le script canonique F8
exporte son profil de compilation. Le crate tty ne requiert aucun SDK natif.
États XDG frères et TMP sur disque court `~/.cache/r3-tty16` créé exclusivement,
CARGO_MANIFEST_DIR runtime fixé au crate tty avant harnais. Aucun credential
ni service partagé ; auth du faux hôte est le jeton factice du harnais.
Empreintes SDK/ELF contrôlées après, scan final puis verdict Common quiescent.
Budget corrigé proposé : **3600s total**, étapes timeout2400 F8 /300 build /
900 harnais dans la même portée, aucun reset ni retry. Sans mesure antérieure
F8 sur ce target léger, ce sont des bornes, pas une durée promise ; dépassement
rouge conservé. Les 7200s antérieures sont retirées, aucun lancement effectué.


### Réserve C7 avant correction du scan (c4ff33a7e5)

Le pilote b70 importe `relocate_target.scan`, qui ignore tous les ancêtres du
contrôleur sans lire exe/cwd/FD/starttime. Un FD de target ou de worktree dans
un ancêtre serait invisible. HOLD GO lourd ; aucune nouvelle exécution et
aucun résultat runtime invalide observé. Porteur tty, même ticket.
Avant correction : publier un témoin discriminant de FD T/W dans un ancêtre,
contrôleur légitime et réutilisation de PID. La frontière corrigée doit lire
chaque processus, y compris contrôleur/ancêtres, ne permettre que les FD des
scripts contrôleurs exacts avec PID/starttime/identité de fichier et programme,
et bloquer tout autre lien attribué T/W. Le lancement sera depuis un cwd
extérieur au worktree ; un cwd W du contrôleur n'est pas une exemption.
Ne pas changer le pilote de relogement historique ni réinterpréter ses preuves.

### Revue Mémoire/C7 b70 — trois réserves P1 regroupées avant correction

1. Scan : ancêtres ignorés sans preuve d'identité, décrit ci-dessus.
2. Provenance : le préflight enregistre les dev/inode/ELF du target actif sans
   les comparer à un attendu figé. L'archive56aca prouve l'ancien rouge relogé,
   pas le target recréé et qualifié6tdyny. À figer : copie/ledger du target actif,
   identités/modes/marker/empreintes, source et reçu vert6tdyny terminé ;
   comparer préparation==entrée après poste avant garde. Un target substitué
   avec le même marqueur ne doit pas passer. Cette capture tardive sera nommée
   telle quelle, sans inventer une empreinte prise au moment de la passe.
3. Finalisation : le VERT dans Common n'est pas le verdict composé. Matérialiser
   lanceur avec budget unique3600 et finaliseur après Common terminé/restes[]/
   nettoye/hash/scan/délai relu. Pas de nouveau budget au dernier pas.

Les trois réserves corrigent un seul pilote, pas trois lots. Publication des
scénarios discriminants avant essais hermétiques ; aucune exécution b70, aucun
Cargo ni nouveau cache. Le test de texte initial du scan est un premier signal,
pas une preuve du comportement : il sera remplacé par de vrais processus factices
avec liens contrôleur/ancêtre/target et starttime changé.

### Correction groupée du protocole, avant toute nouvelle exécution

Les5scénarios hermétiques de `test_target_scan.py`, publiés à b67c9ac84f avant
leur première exécution, passent (0,002s) : contrôleur légitime extérieur sans
exemption PID, FD T/W d'ancêtre/contrôleur bloqué, PID réutilisé détecté,
target modifié/substitué refusé malgré ownertexte identique, finalisation
refusée si portée active/résidu/échéance dépassée/empreinte différente.
Ce résultat ne qualifie pas F8 ni le harnais.

Nouveau lanceur concret `launch_full.py` : prépare uniquement sur sourcepropre,
fige ledger complet target et copiesELF, conserve reçus6td et identité initiale
réelleGO2 (UID/dev/inode/mode), capture tardive nommée comme telle. Les contrôles
sont copiés horsW, depuis un cwd extérieur ; **aucun PID, ancêtre ou FD de W/T
n'est exempté**. Les scripts contrôleurs ouverts sont eux aussi horsW/T.
Un lien W/T dans ces contrôleurs arrête donc comme pour tout autre processus.
Après poste réel, admission calcule le reste du budget original3600 et lance
Common ; préflight exige égalité avec ledger avant garde. Finaliseur extérieur
vérifie Common terminé, code0, restes[], nettoye, empreintes et scan frais,
source propre et temps final avant échéance. Le shell dit STAGES_PASSED,
jamais un VERT composé avant la fin de Common. Aucun GO b70 réactivé.

### Réserve Mémoire sur 291c — script poste encore ouvert sous W

HOLD, aucun run : launch_full.py appelle encore le poste de W. Son interprète
Bash garde un FD sur ce script sous W pendant Common ; le scan sans exemption
doit le refuser avant Cargo. Copier/hasher le poste privé et exécuter sa copie
horsW, après vérification des dépendances du script ; ne pas ajouter d'exemption
W. Réserve de finalisation/publication encore en revue, à regrouper dans le
même ticket et paquet avant nouveau pin. Préparation291c conservée comme telle.

### Fin de revue groupée291c — publication après échéance

Deuxième P1 restante, signalée par Mémoire avant correction : main vérifie
finish(now<END), puis calcule finished non contrôlé et écrit le verdict ;
l'ouverture/écriture peuvent donc dépasser END tout en publiant VERT/exit0.
Prévoir ouverture exclusive et horloge fraîche après open immédiatement avant
publication, ainsi qu'une relecture après écriture ; si tard, verdict ROUGE et
retour1. Témoin sur vraie publication/main, pas uniquement le prédicat finish.
Cette réserve et la copie privée de poste se corrigent dans le même paquet.
Aucun défaut runtime observé, aucune passe291c lancée.

### Fin TOTAL de revue291c : deux P2 complémentaires, même correction groupée

3. Shell lancé depuis Q : git sans `-C W` échoue, et set-u n'arrête pas sur rc
   nonnul. Exiger rc de chaque lecture Git, HEAD==source figée et arbre propre.
4. Finaliseur : comparer les fichiers de hashes produits dans Common ne prouve
   pas que les vraies bibliothèques n'ont pas changé depuis. Réempreinter le SDK
   physique so/vector et les contrôleurs figés après Common ; le snapshot cible
   couvre déjà les ELF, puis appliquer le contrôle frais de publication.

Mémoire confirme FIN TOTAL de cette version : exactement quatre réserves,
deux P1 et deux P2. Aucun autre point en file ; corrections groupées seulement.
ProvenanceT et scan strict291c reconnus bornément, sans autoriser un run.

### Correction groupée des quatre réserves291c

- Copie byte-à-byte et hashée du vrai poste dans Q, invocation Q/poste et
  contrôleurs/cwd extérieurs. Le script n'utilise pas de dépendance relative
  à son ancien dossier : les verrous restent au chemin partagé réel, aucun
  POSTE_LOCK_DIR privé n'est introduit. Aucune exemption W/T au scan.
- Publication réelle exclusive : horloge relue après open avant dump puis
  après flush/fsync ; dépassement force ROUGE et retour1, y compris quand
  l'écriture elle-même consomme le reste. Témoins publiés9a1 avant essai sur
  vrai fichier et sur la queue réelle du main via AST, avec horloge injectée.
- Git toujours -C W, rc obligatoires et source==SHA figée ; témoins publiésa8f
  avant essai depuis cwd extérieur, mauvais SHA et faux worktree refusés.
- SDK so/vector physiques et copies de tous les contrôleurs réempreintés
  après Common, puis délai final et publication. Le test physique modifie
  vraiment une bibliothèque factice, pas un ancien fichier de hashes.

Q/P/S v3 restent conservés ; nouveau paquet v4 uniquement, même target actif.
La préparation v4 doit relire et comparer le ledger et copies du target connu,
sans Cargo, réallocation, suppression, marqueur forgé ni nouveau target.
Les résultats des nouveaux témoins restent NON JOUÉS à cette publication.

### GO distinct C7-TTY-FULL-256-16nov3600 — rouge de préparation conservé

Source/pilote exact `256097281b6588a008858177de7b373576b20608`, Qv4. Budget
original start847833.676762083/end851433.676762083 ; final847834.842848808.
Préflight après vrai poste : ledger1018entrées égal, aucun lien attribué W/T,
4UNKNOWN non attribués consignés ; copies de contrôleurs extérieures à W.

Premier rouge : garde commune lors du passage target crate→workspace, avant
compilation, `cargo metadata` refuse le paquet **ld-lucivy v4.3.9 absent du
cache Cargo**, réseau interdit par CARGO_NET_OFFLINE=true. Message exact :
`failed to download ld-lucivy v4.3.9` puis
`attempting to make an HTTP request, but --offline was specified`.
Ce n'est pas un échec d'assertion ni un défaut produit établi. F8rc2/pilote1 ;
aucun juge F8 joué, aucun bâti tty ni harnais lancé, aucun retry/reset.

Preuves sous `~/.cache/rag3weaver-build/` :
- `cx-tty-english-aliases-full-v4/{f8.log,f8.rc,profile-before.json,sdk-sha.txt}` ;
- `cx-tty-aliases-controller-v4/{allocation,remaining-after-poste,composed-verdict}.json` ;
- `cx-tty-aliases-common-v4/validation-zcvv4qs0/` : ROUGE code1/erreur null,
  restes[], nettoye true ; autorité PID539556/start84783370 TERMINÉE.

Worktree propre après et marqueur target toujours propriétaire crate tty ;
la garde n'a pas effectué l'adoption. Les verts crate0c4 restent historiques,
aucun vert complet pour le candidat. Prochain pas transmis C7 : préparer
l'acquisition de la dépendance exacte, avec provenance et voie réseau ciblée
validées avant une nouvelle allocation ; aucun téléchargement ni reprise
implicite. Target et preuves restent conservés.

### Préparation réseau ciblée autorisée, avant toute acquisition

Autorisation C7 après STOP256 : uniquement ld-lucivy4.3.9 verrouillé,
crates.io, staging privé horsT, checksum/extraction stricte, ajout atomique
sans écrasement des seules entrées absentes. Aucun Cargo/build/test/autorité.
Cargo.lock SHA256 `090cc4e1330dc694127c0f08330d25acc656699763079400b502ba6607093fa7` ;
source registry+https://github.com/rust-lang/crates.io-index ; package checksum
`24424a98439ff03e1fb75fc749a4d88d720ca5b9084b6c1a885dcd6cfe99afb7`.
Chemins cache et src exacts sous
`~/.cargo/registry/{cache,src}/index.crates.io-1949cf8c6b5b557f/` vérifiés absents
pour ld-lucivy4.3.9, sans lien. Le script préparé vérifiera encore leur état
avant acquisition/publication. Le payload .crate seul sera ajouté au cache ;
l'extraction sera vérifiée en staging privé, le src Cargo restera à Cargo.

Inventaire filesystem des autres payloads verrouillés manquant à la fois en
cache et src : **luciole4.3.9, lucistore4.3.9, lucivy-core4.3.9,
sparse-vector4.3.9**. Liste transmise avant tout élargissement ; aucun de ces
quatre paquets n'est autorisé implicitement par l'acquisition ld-lucivy.
Pas de cargo fetch massif ni de version flottante, pas de SDK/guard changé.

### Acquisition ciblée ld-lucivy terminée, sans relance

Script a0bca6925a publié avant action. URL exacte
https://static.crates.io/crates/ld-lucivy/ld-lucivy-4.3.9.crate ; 1311506octets,
SHA verrouillé24424a98439ff03e1fb75fc749a4d88d720ca5b9084b6c1a885dcd6cfe99afb7.
419fichiers extraits et rehashés en staging, chemins relatifs/préfixe unique,
liens et objets spéciaux refusés, limites taille/nombre, nom/version manifest
vérifiés. Preuve `cx-tty-crate-ld-lucivy-439/proof.json` sous le cache privé.
Seul le .crate final absent a été ajouté par renameat2 NOREPLACE ; UID1000,
dev57/inode94970729/mode0644/hash relus. SrcCargo demeure absent et aucun
fichier préexistant n'a été modifié. Aucun autre paquet acquis.

RAW256 P/Q/scope conservés aux deux postes dans
`archives/cx-tty-full-256-raw/evidence.tar`, SHA256 comparé identique
`3b84d438f0555fba63796851848d17654c119d62def58fb05dad842fdb38205c`.
Aucun Cargo/metadata/build/test/service/autorité ni réessai après préparation.
Les4autres absences ont été signalées avant toute acquisition élargie.


### Élargissement ciblé autorisé, pins AVANT chaque acquisition

C7 autorise les5paquets suivants uniquement, lock090cc4 inchangé ;
aucun Cargo/metadata/build/test/autorité pendant cette préparation.
- `ld-lucivy 4.3.9` : `24424a98439ff03e1fb75fc749a4d88d720ca5b9084b6c1a885dcd6cfe99afb7` (registry+https://github.com/rust-lang/crates.io-index).
- `luciole 4.3.9` : `9a9f91d0e6326601e8c7c2d3390af3b2d5199c459d3bc5a1d9a3200d2969d0f9` (registry+https://github.com/rust-lang/crates.io-index).
- `lucistore 4.3.9` : `7ccae99afa56dc99810a24f1c322ebb94b5962a0adf9a738576c1b565f92b8a2` (registry+https://github.com/rust-lang/crates.io-index).
- `lucivy-core 4.3.9` : `8343104e0dfec41a1341f8c5092cd0cf3e5dca0b98d07d00f1a8ceca5e11c71a` (registry+https://github.com/rust-lang/crates.io-index).
- `sparse-vector 4.3.9` : `8d135db2852e30d5d05b412c6a6e4f161d2ee52168ed7e3e5bc81348e8071a7c` (registry+https://github.com/rust-lang/crates.io-index).

Staging distinct par paquet ; cache .crate seul, création atomique si absent.
Si déjà présent ou concurrent, valider hash/type/UID sans écraser.

### Préparation cinq paquets terminée, nouveau pin distinct

Les5.crate exacts sont présents et rehashés conformes au lock090cc4 :
ld-lucivy déjà présent seulement vérifié ; quatre autres ajoutés absents
atomiquement, chacun UID1000/dev57/mode0644 et SHA consigné. Extraction
stricte privée pour chacun, aucun srcCargo écrit, aucun fichier remplacé.
Rapport `cx-tty-five-crates-preparation/report.json` FIVE_VERIFIED, inventaire
filesystem lock complet missing_locked_payloads=[] ; pas une preuve metadata.
Aucun Cargo, metadata, build, test ou autorité exécuté pendant l'acquisition.

Master relu b0a845bb4877328eada37951c174e3d208986630 inchangé depuis l'union.
Nouveau paquet Q/P/S v5 et TMPcourt v5 ; le contrôleur vérifie les5payloads et
le reçu rouge256 le plus récent terminé/restes[]/nettoye, en plus du reçu
vert6td historique ; target actuel exigé identique au ledger d'avant le STOP256.
Aucun ancien budget repris. Préparation v5 puis RO/GO distincts nécessaires.


### GO C7-TTY-FULL-6D8-16nov3600 — F8 acquis, refus de préparation tty

Source/pilote exact `6d886375ea4627c88d28aef918b948a23f628ab1`, Qv5.
F8 canonique **CHECK_FEATURES VERT 8/8** lu, les8configurations compilées.
Puis vraie garde pour revenir au crate tty : cargo metadata offline/locked
refuse `crossterm_winapi v0.9.1` absent du cache, avant tout bâti tty.
Buildrc2/pilote1, premier refus arrête sans retry. Harnais NON JOUÉ.
Ce n'est pas un défaut d'assertion ni un ticket clos sur F8 seul.

Preuves privées `cx-tty-english-aliases-full-v5/{f8.log,features/,build.log,build.rc}`,
`cx-tty-aliases-controller-v5/composed-verdict.json`,
`cx-tty-aliases-common-v5/validation-93l3ksmi/` : ROUGE code1, erreur null,
restes[], nettoye true, autorité574793/start84848282 TERMINÉE.
Budget start848482.791694931/end852082.791694931/final848934.497279601,
soit451,71s, aucun reset. Source propre. MarqueurT désormais workspace
rag3weaver après F8 réussi ; retour vers tty NON effectué après refus metadata.

Inventaire filesystem de **tous** les absents cache+src du lockTTY séparé,
SHA9127845d654a37f4a9e0fbce59b69ef26da51c6f6d7af6a6833782fd4fecabbb :
`docs/tty/16-11-2026/tty-lock-missing-after-6d8.json`, versions/checksums/source.
Huit paquets : crossterm_winapi0.9.1 ; js-sys0.3.107 ;
wasip2 1.0.4+wasi-0.2.12 ; wasm-bindgen, wasm-bindgen-macro,
wasm-bindgen-macro-support et wasm-bindgen-shared0.2.130 ; wit-bindgen0.57.1.
Aucun téléchargement de ces8paquets, aucun metadata supplémentaire.
Liste transmise C7 avant tout élargissement ciblé. Prochaine recette doit
préserver F8 acquis et nommer le propriétaireT réel avant un GO neuf.

### Préparation de continuation autorisée, sans rejeu F8 ni budget neuf

C7 autorise après ticket tous payloads manquants verrouillés du **lock tty**
SHA9127845d654a37f4a9e0fbce59b69ef26da51c6f6d7af6a6833782fd4fecabbb,
inventaire complet avant actions et staging/hash/extractionstricte/cacheabsent
atomique pour les8pins déjà publiés. Aucun Cargo/build/native pendant prépa.
Source produit6d figée ; continuation buildTTY+harnais seulement. F8 porté
avec ses8logs/hash/sourceproduitidentique, pas rejoué. Target propriétaire
workspace root réel, reçu93l terminé, ledger nouveau à figer avant sa garde.
NouveauxP/Q/S/TMP, mais **même INITIAL848482.791694931 et
END852082.791694931**, minreste calculé après vrai poste ; si expiré, fermer.
Lecture de l'horloge avant prépa : now849100.284, reste2982,5s, non réinitialisé.

### Huit payloads tty préparés et continuation sans F8

Préparation18a7 publiée avant actions : inventaire **complet** du lockTTY,
8absents téléchargés aux versions/checksums verrouillés, chacun extrait et
vérifié strictement en staging, .crate ajoutés absents par NOREPLACE/UID/mode
consignés. Rapport `cx-tty-lock-payload-preparation/report.json`
TTY_LOCK_PAYLOADS_READY/missing_after[] ; aucun Cargo/metadata/build/test/natif.

Pilote de continuation distinct `launch_continuation.py` et
`check-english-aliases-continuation.sh` : uniquement buildTTY+harnais, source
produit6d prouvée identique par diff Git de tout extension/rag3weaver, F8 porté
par les8logs et leurs SHA/source/rc0. Pas de rejeu F8. Q/P/S/TMP de continuation
neufs, target unique propriétaire root réellement capturé et ledger figé.
Le reçu93l terminé et le profil F8 restent conservés ; vraie garde root→tty
au prochain créneau seulement. Les13payloads des deux locks sont vérifiés.

INITIAL848482.791694931/END852082.791694931 conservés dans le contrôleur :
préparation et file consomment le même budget original, timeout extérieur et
Common réduits au reste. Expiration ferme le lot au lieu de recréer3600s.
La préparation n'est pas une autorisation de lancement ; RO et GO distincts.

### Continuation C7-TTY-CONT-9D8-16nov-RESTE — bâti acquis, harnais rouge

Sourcecontrôleur `9d83118b19aba9f2c4793dfe0337026b8fad4be4`, produit6d
identique ; F8 porté8/8, **pas rejoué**. Garde réelle root→tty réussie,
bâti tty **rc0 en1,07s**. Reçu continuation après poste : reste829s sous
INITIAL848482.791694931/END852082.791694931 originaux, aucun reset.

Premier rouge lu dans le harnais : après accord `o`, aucune fin attendue
(anim reposdaemon ou « fini sans répondre ») détectée. Arrêt de l'autorité
Common **propriétaire seulement**, via pidfd après vérification exacte
PID705646/start85125068/cgroup/scriptQ/état actif. Preuve
`cx-tty-aliases-continuation-v1/first-red-stop.json`. Le superviseur a arrêté
ses descendants ; aucun processus tiers ni service partagé touché.

Deuxième observation déjà écrite avant arrêt effectif : `multiligne_kitty`,
indicateurs d'aide/kitty absents malgré réception du texte un/deux/trois avec
les deux sauts de ligne attendus. Ces constats restent à attribuer : aucun
correctif produit ni réessai lancé, pas de conclusion de non-régression.
Le harnais **ENTIER est NON ACQUIS**, interrompu au premier rouge observé.

Preuves privées P/Q/scope de continuationv1 : build.log/rc, harness.log,
first-red-stop.json, source/ELFhash, configuration/reçus. Common
`validation-flsb10fe` ROUGE code1/signal15 de cet arrêt explicite,
restes[], nettoye true ; autoritéTERMINÉE, pic320704512octets, oom_kill0.
Composé ROUGE, final851326.668131734 avant ENDoriginal. Worktree propre,
markerT maintenant propriétaire crate tty. Même target conservé, aucun retrait.
Prochain pas : lecture des traces et attribution des deux observations avant
toute nouvelle action autorisée ; ticket reste ouvert, aucun push master.

### Diagnostic RO après interruption, sans attribution abusive aux aliases

Analyse et plan de diff, preuves manquantes explicitement nommées :
`docs/tty/tickets/2026-11-16-le-harnais-perd-les-traces-des-seances-et-juge-la-fin-sur-les-octets.md`
(porteur tty). Premier rouge = terminal riche après accord, pas le contrat
--plain ended/relance ; le ticket plain reste distinct. Deuxième observation
rattachée au ticket tmux15 existant, texte multiligne correct reçu, aide non
retrouvée dans le flux brut. L'hôte927 n'a pas été appelé, API réelles non jugées.
Aucun raw terminal/capture/frappe après arrêt inventé ; les chunks étaient en
mémoire et le faux hôte écrase son journal entre séances. Analyse de source
seule, aucun essai, metadata, provider, service, correctif ni nouvelle allocation.

### Harnais934F vert mais raccord dynamique anglais restant, avant transmission

Harnais ENTIER VERT/composé VERT0/source934f, F8/build portés, preuves et
limites dans `docs/tty/16-11-2026/05-bilan-alias-et-harnais.md`. Ne pas fermer
ce ticket sur ce seul vert de compatibilité : sources_de dans tui.rs ne charge
le catalogue que pour les préfixes /tache et /depot. Action::Completer lui
transmet /task et /repo non normalisés ; commandes::completer sait les traiter,
mais les candidats restent vides. Les tests unitaires injectaient Sources,
ils n'exerçaient pas ce raccord. Aucun défaut d'exécution des commandes déduit.

Porteur tty, même ticket ; réservation complémentaire **tui.rs nécessaire**
conformément à l'annonce initiale. Plan : témoin du vrai chemin de collecte,
rouge avant correctif, puis baser la décision de collecte sur le parseur
commandes::lire existant plutôt que maintenir une liste de préfixes parallèle.
Aucun fix ni nouveau test/HTTP/Cargo lancé à cette découverte. SourceRust
934f/F8/harnessgelés comme preuves, aucune autorisation nouvelle déduite.

### Réservation TUI et union99, témoin avant fix

C7 autorise tui.rs uniquement sources_de/ActionCompleter+tests du vrai collecteur.
NouveauW cx-tty-english-aliases-union-99/branchecx-tty-english-aliases-7 base99fef92ce6,
ancien934/W/T figés. Audit et plan de qualification
`docs/tty/16-11-2026/06-union-master-99-et-reste-tab.md`.
TémoinRust préparé : vrai sources_de avec capacité load_state, ActionCompleter
lui fournit client::lire_brut ; spy de la route et completion réelle pour4formes
partielles FR/EN. Le branchementFR est volontairement intact, aucun fix anglais.
NON JOUÉ, aucun Cargo/HTTP/metadata. CouponROUGE600 proposé, pilote exact et
RO/GO distinct requis ; F8old6d ne qualifie pas cette union avec4entrantsVertex.

### Pilote rouge du vrai collecteur, avant RO/GO

Nouveaux tab_red.py/admit/driver : coupon600s total/file+Common+final/pub sous
watchdog extérieur déjà qualifié,32G/j4, un test cratevue exact/spyroute,
aucun HTTP serveur/7970/SDK/provider. Préparation : nouveauW99/sourcepropre,
target unique ancienWownercapturé/ledger/quiet/reçu934TERMINE et archive des
empreintes locales avant toute adoption. Garde commune via cargo_garde réel
sous poste vers nouveauWtty ; pas de .arbre forgé ni nouveau target.

Classifier ne confirme le rouge que si Compiling depuis nouveauW et1test joué,
rc101/0passé/1échoué/0ignoré/cause catalogue request for /task mes. Toute erreur
de préparation/compilation n'est pas le rouge attendu. Nouveau testELFhash et
reçuTERMINÉ/restes[] conservés, composé ROUGE attendu/exit1, jamais un VERT.
F8old6d/ELFcb24 sont exclusivement historiques, jamais juge de l'union99.
NON JOUÉ ; source/pilote/paquet RO puis GO C7distinct obligatoires avant Cargo.

### Réserve de RO041660, avant correction du témoin

Mémoire fin totale : un refus statique bloquant. Le remplacement du point
catalogue a touché aussi connaitre(), qui appelle load_state sans paramètre
ni binding, donc E0425 avant le juge. Ce n'est pas le rouge d'assertion demandé
et aucun Cargo n'a été lancé. Rétablir client::lire_brut dans connaitre seulement,
conserver seam ActionCompleter/sources_de et préfixesFR inchangés ; nouveau
Q/source pour ROdelta. AncienQv1 et target restent conservés.

### 17 novembre : rouge du vrai collecteur BC8 réellement acquis

GO C7-TTY-TAB-BC8-17nov600/sourcebc8ab075813cb882c9876dffbc2f3bb8128da560,
base99, worktreeunionneuf. Compilation depuis ce chemin attestée ;1test exact,
0passé/1échoué/0ignoré,113filtrés,Cargo101. Assertion précise
`catalogue request for /task mes` gauche[]/droite[/ide/etat], après les2casFR.
Nouveau testELFSHAc7d834…76b4f2 ; pas d'HTTP réel ni serveur, aucune erreur
compile présentée comme rouge. Composé ROUGE_ASSERTION_CONFIRME/rc1,
Commonp8xp1kn3TERMINE/ROUGE1/erreur null/restes[]/nettoye true,8,169s<600.
Quietfound[]/4UNKNOWNnonattribués, source propre ; markerT désormaisWuniontty.

RAW2hosts archives/cx-tty-tab-bc8-red/evidence.tar SHA6f1ab9a07133b659d58de384079f3a9556f0f18838088427d0940f23bc4d1f13,
preuves choisies/rapport `docs/tty/17-11-2026/01-rapport-de-session.md`.
Aucun fix/retry/secondtest/F8/provider/HTTP/SDK/PCrun/master ; ancien934/RAW/ELF
conservés. Correction et qualification suivantes attendent les consignes C7.

### Correctif produit préparé après rouge confirmé, pas rejoué

C7 autorise après archivesBC8 rehashées2hosts et ticket publié9eab76ff61 :
`source_de` compose désormais commandes::lire AVANT Sources, décide les
capacités par Modele/Tache/Depot canoniques. Pas de seconde table de préfixes,
arguments/aliasFR conservés par le parseur déjà livré. ActionCompleter et
le juge réel sont textuellement inchangés, hash/audit
`docs/tty/17-11-2026/tab-fix-judge-audit.json`.
Aucun test, Cargo, build, metadata, F8, service ou HTTP rejoué sur l'ancienGO600.
Le target reste propriétaire nouveauWtty avec le testELFrougeBC8 ; nouveau
coupon vert/source/pin et RO/GO distinct avant qualification.

### Coupon vert distinct préparé, pas exécuté

Nouveaux tab_green.py/admit/driver : même test exact/juge inchangé,600sTOTAL
OS/poste32G/j4/Commonreste/final/pub ;1passé/0échoué/0ignoré+compilationdepuisW
obligatoires, sinon refus. Verdict VERT_CIBLE uniquement, pas passe complète99.
Préparation garde le rougebc8/source/ELFc7d834/ledgerpost/reçuTERMINE et copie
ce testELFavant recompilation. Towner attendu déjà nouveauWtty ; vraie garde
cargo_garde et aucun nouveau target, préfixe ancienT non repris artificiellement.
Pas HTTPserveur/7970/SDK/provider ; sourcecallbacks ActionCompleter inchangée.
Aucun ancien600 réactivé. Source/pin/Qneuf puisRO/GOdistinct avant Cargo/test.
