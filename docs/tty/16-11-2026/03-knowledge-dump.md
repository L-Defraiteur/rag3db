# Knowledge dump tty — 16 novembre 2026

Ce qu'une session neuve doit savoir pour travailler sur le terminal de l'agent (`extension/rag3weaver/tty`). Remplace, pour le terminal, le dump du 11 novembre (`docs/tty/11-11-2026/02-knowledge-dump.md`), qui reste vrai pour ses débuts.

## Le crate

| Fichier | Rôle |
|---|---|
| `src/main.rs` | les options, le choix vue / texte brut, le mode `--plain` (boucle du flux et relecture de la boîte) |
| `src/tui.rs` | la vue riche : abonnement au flux, filtre par tâche, boucle des touches, exécution des `/commandes` et des `!commandes`, routes de l'hôte appelées |
| `src/vue.rs` | l'état affiché : l'historique validé, la zone vivante (ligne discrète, ligne d'état, saisie), les règles d'affichage ; la plupart des tests |
| `src/boite.rs` | les attentes (question, approval) et l'en-tête français d'une ligne de boîte (`en_tete`) |
| `src/outils.rs` | les appels d'outil (`Appel`), leurs aperçus, le découpage Markdown en blocs |
| `src/visionneuse.rs` | Ctrl-O : un appel en grand, en écran alterné |
| `src/anim.rs`, `src/theme.rs` | les animations et les thèmes, lus de `templates/themes/*.json` (`include_str!`) |
| `src/commandes.rs` | la lecture des `/commandes` et `!…`, l'aide, la complétion par Tab |
| `src/coquille.rs` | une `!commande` dans un pseudo-terminal (FFI libc), `sudo` compris |
| `src/saisie.rs` | l'éditeur de ligne multi-ligne |
| `src/plain.rs` | le rendu texte brut |
| `src/client.rs`, `src/flux.rs`, `src/sse.rs`, `src/endpoint.rs` | l'hôte : HTTP, flux SSE `/ide/flux`, `daemon.json` |
| `src/journal.rs`, `src/historique.rs` | `--journal` (ce qui part dans l'historique, en texte) ; l'historique des envois entre séances |
| `mesures/mesure.py`, `mesures/faux_hote.py` | le harnais et le faux hôte |
| `mesures/plain_stays_attached.py` | le témoin du mode `--plain` (T1) |

La feature `vue` (par défaut) apporte ratatui ; `--no-default-features` ne garde que le client et `--plain`, avec ses propres tests (20).

## Les règles de la vue (ce que Lucie a demandé, ne pas les défaire)

- **L'historique ne garde que la demande et la réponse.** Les appels d'outil défilent sur la ligne discrète (`Vue::discret`), sans verdict (« fait (3 ms) » ne s'affiche pas, un refus si). La plomberie du pas (`step_done`, `report`, `edit_plan`), les étapes faites et les mots de l'hôte ne s'affichent pas. `--etapes` ou `/etapes` remontre tout.
- **Jamais un mot d'état nu.** Au travail : l'animation du thème (`Quelle::Reflechit`, `Lit`, `Ecrit`), avec la cause à côté. Pendant que le texte coule : le cœur seul (« ⛧ … »). Au repos : l'animation `repos` du thème. Une phrase seulement pour une fin sans réponse ou une erreur. `RunStarted` lance l'animation, parce que l'hôte n'émet `LlmCall` qu'au retour du modèle.
- **Les accords sont vivants.** Une demande d'accord ne s'écrit pas ; la ligne d'état la dit avec `[ Oui ]   Non` (Oui choisi d'avance, ← → et Entrée, `o`/`n`), jamais recouverte par une animation, encore dite après la fin du pas ; la réponse, du terminal ou de l'IDE, laisse une ligne « › accordé : … » ou « › refusé : … ». Une proposition de déclarations du dépôt principal a les mêmes boutons.
- **Une ligne s'écrit une fois** :
  - par `id` ;
  - la copie de la réponse que l'hôte pose à la fin du pas (texte nu, ou derrière « le pas N s'est terminé sans step_done ; sa réponse : ») ne se réaffiche pas si elle égale le texte streamé (`texte_coule`) ;
  - ce que tu envoies d'ici est écrit tout de suite (« › … »), et sa copie de la boîte ne se réécrit pas (`envoyes`).
- **Le nom de l'agent** : une ligne `from: "teammate"` qui porte `name` s'en-tête par lui ; la vue retient le dernier nom vu, ou celui que `/nom` a fait garder.

## L'hôte, tel que le terminal le lit

- `daemon.json` (`~/.local/share/rag3weaver/daemon.json`) : adresse et jeton.
- Le flux `GET /ide/flux?sujets=agent,tokens,mailbox,declarations` (en-tête `Last-Event-ID` pour reprendre ; un événement `lacune` quand le tampon a perdu des trames). Enveloppe : `{id, sujet, backend, at_ms, evenement: {kind, …}}`. Le `backend` est le chemin du manifeste du worktree de la tâche : le terminal filtre sur lui (`v["backend"]` de `GET /ide/teammate`).
- `GET /ide/teammate?backend=<id>&task=<nom>` (boîte entière, manifeste) ; `POST /ide/teammate/mailbox` (demande, consigne, réponse, accord) ; `GET /ide/teammate/list?supervisor=supervisor` ; `GET /ide/teammate/models`, `POST /ide/teammate/model`, `POST /ide/teammate/name` ; `POST /ide/propositions/appliquer` et `/rejeter` ; `GET /ide/chat?backend=…` (contexte) ; `GET /ide/etat` ; `POST /ide/describe` (politiques des outils) ; `POST /highlight`.
- Une réponse vaut `{ok, result}` ; le client (`client::rendu`) rend `result`, ou une erreur « POST <chemin> : <error> (<statut>) ». Une route inconnue rend 404 « route inconnue : … ». Certains refus arrivent en 200 avec `ok: false`.

## Le harnais

- `mesures/mesure.py <hôte|local> <binaire> <faux_hote.py> <dossier> <themes.json> <sortie.json> [lanceur]`, lanceurs `ssh`, `tmux-ssh`, `tmux-local`, `tmux-local-ouvert` (tmux réglé en CSI u), `mosh`. Sous `tmux-local-ouvert`, le harnais entier doit être VERT. Il dit VERT ou ROUGE en dernière ligne.
- Le `themes.json` qu'il lit est un dictionnaire de tous les thèmes : le refaire après tout changement de thème :

      python3 -c "import json,glob,os; print(json.dumps({os.path.basename(p)[:-5]: json.load(open(p)) for p in glob.glob('extension/rag3weaver/templates/themes/*.json')}, ensure_ascii=False))" > <dossier>/themes.json

- Le faux hôte (`faux_hote.py <port> <journal.jsonl> <daemon.json>`) sert ce que lit le terminal ; à une demande, il joue un pas scripté (tokens, bloc de code, `read_file`, `edit_file`, puis une demande d'accord). Routes pour les témoins : `/faux/ligne` (une ligne de boîte d'un auteur donné), `/faux/marque` (un token tout de suite).

## La passe courte d'un lot du terminal

La passe de référence est `tools/poste/passe_epinglee.sh` (voir `docs/procedures/push-d-un-lot.md`) ; voici en plus ce que tty a joué avant chaque push (16 nov.), en script sous `poste lourd`. Remplacer `W` par le worktree du lot ; il écrit `<dossier>/resume.log`, qui finit par VERT ou ROUGE :

```bash
#!/bin/bash
set -u
W=$HOME/git_workspaces/<worktree>; J=$HOME/.cache/rag3weaver-build/$1; mkdir -p $J; R=$J/resume.log
export CARGO_BUILD_JOBS=8
export RAG3DB_LIB=<nom de la lib épinglée, voir docs/procedures/push-d-un-lot.md>
echo "commit $(git -C $W rev-parse --short=9 HEAD) — $(date '+%d/%m %H:%M')" > $R
rouge() { echo "ROUGE $1" >> $R; exit 1; }
cd $W/extension/rag3weaver && . scripts/lib_du_magasin.sh
exporter_la_lib > $J/lib.log 2>&1 || rouge "exporter_la_lib"
cd $W/extension/rag3weaver/tty
export CARGO_TARGET_DIR=$HOME/.cache/rag3weaver-build/target-tty-wt/$(basename $W)   # un target à ce worktree
timeout 1800 bash -c 'touch src/lib.rs; cargo test -j 8 && touch src/lib.rs && cargo test -j 8 --no-default-features' > $J/crate.log 2>&1 || rouge crate
grep -E '^test result' $J/crate.log >> $R
[ "$(grep -c "Compiling rag3weaver-tty v.* ($W/extension/rag3weaver/tty)" $J/crate.log)" = 2 ] || rouge "pas compilé depuis $W"
cd $W/extension/rag3weaver
export CARGO_TARGET_DIR=$HOME/.cache/rag3weaver-build/target-tty-features/$(basename $W)
scripts/check_features.sh > $J/check_features.log 2>&1 || rouge check_features
tail -1 $J/check_features.log | grep -q '^CHECK_FEATURES VERT' || rouge check_features
echo VERT >> $R
```

À ajouter selon le lot : `python3 scripts/themes_palettes.py --check` et le témoin des thèmes de l'hôte (`every_templates_theme_is_served_and_an_unknown_one_is_404`) si un thème change ; le harnais entier si l'affichage change ; `mesures/plain_stays_attached.py` si `--plain` change ; la lib entière sous le jeu que donne la procédure (`rag3db-native,burn-embedder,burn-ocr,code,daemon,postgres,openai-llm,anthropic-llm`, `RAG3WEAVER_PG` posé) si un fichier de l'hôte change. Lancer : `cd ~/.cache/rag3weaver-build && POSTE_MEM_MAX=32G ./poste lourd timeout 5400 ./<script> <dossier>`. Écrire la base (`git rev-parse` du parent du premier commit du lot) dans `<dossier>/base` avant `poste push`.

## Les pièges rencontrés

- **Target partagé** : deux worktrees sur un même target → cargo rejoue le binaire de l'autre arbre sans « Compiling ». D'où la vérification de la ligne `Compiling … (<worktree>)`.
- **Un filtre d'affichage qui ment** : `grep -v "0 passed; 0 failed; …; 0 filtered"` masque aussi « 20 passed; 0 failed… ». Lire le `resume.log` entier.
- **Les rebases sur `docs/nouveautes.md`** conflictent à chaque lot (une entrée en tête). Garder les deux côtés et ne rien toucher d'autre ; vérifier `git diff origin/master -- docs/nouveautes.md` après, puisqu'un résolveur a déjà doublé une entrée et ôté des lignes vides de master.
- **Les tests de l'hôte** qui déclarent des modèles `compatible` exigent `feature = "openai-llm"` (tty-17).
- **Le plafond d'octets** par token (`tui::tests::mesure_des_octets_pour_mille_tokens`, 35 000 octets pour 1 000 tokens) : une animation pendant le texte qui coule le dépasse. Ne pas relâcher le seuil.
- **Annoncer un vert sans l'avoir joué** : « /nom marche » avait été dit sans l'essayer contre un hôte qui n'avait pas encore la route. Jouer contre l'hôte réel, sur une tâche à soi, avant de dire « marche ».
- **Le préfixe français de l'hôte** est reconnu par son texte (`sans_mot_d_hote`) ; voir le ticket du 16 nov. sur le champ.
