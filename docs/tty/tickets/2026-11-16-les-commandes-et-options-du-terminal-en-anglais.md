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
