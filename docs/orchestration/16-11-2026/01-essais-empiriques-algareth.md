# Essais empiriques d'Algareth, par c7

Lucie, 16 novembre 2026, 0 h 5x : « joue mon propre rôle jusqu'à ce qu'on sache vraiment s'en servir comme
autre session ; fais-le sans moi, ça devrait être ça vos tests : pas unitaires, empiriques ; contrôler chaque
outil de recherche, s'il a bien marché, etc. »

Protocole : une tâche à c7 sur le backend de Lucie (`c7-essai`, agent Algareth), le terminal en mode `--plain`
avec `--demande` et `--journal`, une demande à la fois ; pour chaque demande, l'attendu est écrit **avant**, puis
les appels d'outils relevés dans le journal (nom, arguments, retour) et la réponse ; le verdict est VERT si
chaque outil a fait ce qu'il dit et si la réponse est juste, ROUGE sinon, avec la cause et le ticket chez la
session qui porte la couche. Les demandes vont du plus simple (une recherche) au plus long (un témoin rouge puis
vert sur un défaut cadré).

| n° | demande | attendu | outils relevés | réponse | verdict |
|---|---|---|---|---|---|
| D1 | « Dans quel fichier et à quelle ligne est définie `valider_le_nom_de_l_agent` ? chemin relatif et ligne, rien d'autre » | `extension/rag3weaver/src/backend.rs:995` par `search_code` ou `usages` (l'index est prêt), un ou deux appels | grep_files ×5, read_file ×4, run_command `git rev-parse --show-toplevel` (accord demandé, resté en attente : « en attente du superviseur depuis 00:32 »), list_files ×1 ; **aucun search_code, outline ni usages**, alors que la tâche les a | « Je suis Algareth. extension/rag3weaver/src/backend.rs:995 » ; l'hôte dit « le pas 1 s'est terminé sans step_done » | **VERT sur la réponse, ROUGE sur la voie** : 11 appels dont 9 lectures brutes pour une définition que l'index donne en un ; une commande en lecture seule bloquée à l'accord en mode plain ; se présente dans une réponse qui ne le demande pas. Tickets : archi (l'index d'abord, grep en repli ; le pas sans step_done dit comme un défaut) ; porte (un `git rev-parse` en lecture seule passe en auto, vision point 6) |
| D2 | « Qui appelle `valider_le_nom_de_l_agent` ? fichier et ligne » | `usages` → backend.rs:1008, taches.rs:554, taches.rs:574 | pas 3 : `usages {name}` → exactement les trois (call 2, other 1) ; puis grep_files et deux read_file de confirmation ; `step_done` sans report → relance ; pas 4 : grep encore, report, step_done | les trois, exacts | **VERT** sur l'outil et la réponse (47 s). Défaut : step_done avant le report (relance, +12 s), grep redondant après usages |
| D3 | « fonctions publiques de daemon_host/taches.rs, noms seulement » | par `outline` ; 13 `pub fn` | grep_files `pub (async )?fn` → 13 ; `pub.*fn` → 18 ; `outline {path}` → 65 scopes avec signatures (porte `pub`) ; grep ; read_file ; report ; step_done | « Je suis Algareth. » + les 13 exacts | **VERT** sur la réponse (13/13). `outline` appelé après le grep et n'a servi à rien ; se présente de nouveau (3ᵉ fois). 48 s |
| D4 | « recopie les lignes 995 à 1003 de backend.rs » | `read_file` par offset, texte exact | pas 1 : `read_file {offset 995, limit 15}` ; step_done sans report → relance ; pas 2 : **le même** read_file, report, step_done | bloc rust des 9 lignes | **VERT**, identique au bit près à `sed -n 995,1003p` de master. Même défaut step_done avant report (+15 s). 29 s |
| D5 | « où est décidé le nom par défaut d'un agent : fichier, constante, valeur » | `search_code` → `NOM_DE_L_AGENT_PAR_DEFAUT` = « équipier » | grep `agent` (5 122 résultats, 50 montrés), grep `NOM_` (coup de chance), read_file ; step_done sans report → relance ; pas 2 : grep `DEFAULT_.*NAME\|default.*name\|manifest` (4 881), **grep « Algareth »** (53), deux read_file, grep, report | backend.rs:1012, la constante, « équipier » | **ROUGE sur la voie, réponse juste** : question de sens, `search_code` jamais appelé ; deux greps à plus de 4 800 résultats ; après la relance tout refait de zéro (les retours d'outils du pas d'avant ne lui sont plus montrés) ; grep sur son propre nom. 63 s |
| D6 | la tâche complète du nom avec un blanc (test rouge, correctif, vert, diff, « prêt »), « continue jusqu'au bout » | test écrit, cargo test rouge puis vert, diff | 4 lancements (demande + 3 « continue »), **7 pas, 82 appels : 33 grep_files, 31 read_file, 13 run_command (git status/log/show), 3 list_declarations, 1 proposals, 1 step_done, 0 edit_file, 0 cargo**. Chaque pas : ~11 lectures puis « This is your last step: no more tool calls are possible », réponse « point d'étape / ce qui reste à faire », pas de step_done, puis l'agent reste idle jusqu'à la relance ; 3ᵉ lancement : l'hôte ferme « 3 pas de suite sans élément réussi » ; 4ᵉ : step_done avec remaining → pas suivant, encore 11 lectures, idle | « Je suis Algareth. Voici le point… Ce qui reste à faire au prochain pas… » ×4 | **ROUGE** : `git status --porcelain` vide à 1 h 11, **pas une ligne écrite en 20 min** (4 min 40 s de modèle). Le rouge connu d'archi, aggravé par S1 à S4 |

Temps : D2 47 s, D3 48 s, D4 29 s, D5 63 s, D6 abandonnée après 20 min (chaque pas 41–57 s, 95 % de modèle ; les outils répondent en 100 à 2 000 ms).

## Où sont tracés les appels d'outils

Le journal des runs de l'hôte : `~/.local/share/rag3weaver/runs/<backend de la tâche>/agent-<run>.json.zst` (ici `e9c92e8594cc`, l'id du backend de la tâche, pas celui du parent). Chaque fichier porte `tours[]` (la conversation reçue par le modèle, chaque `tool_calls[].{name, arguments}` et chaque retour `role: tool` en entier) et `evenements[]` (LlmCall avec modèle, jetons, ms ; ToolCallStarted ; ToolCallFinished avec ms, octets, extrait). Écrit à la fin de chaque run racine (journal_des_runs.rs ; `tours` ajoutés par chat.rs:797). Lecture : `zstd -dc | python3`. Deux limites : le champ `demande` porte toujours le texte de D1 pour D2 à D6 (J1) ; rien n'est écrit tant qu'un pas n'est pas fini (un pas en cours est invisible, sauf `GET /ide/chat?backend=…` → `run`). `journalctl --user -u rag3weaver-daemon` est vide ; `~/.local/share/rag3weaver/chats/<parent>/` est vide. La boîte (`mailbox.jsonl`) ne porte pas les appels d'outils.

## Les défauts, par couche

**Socle et pas (archi)**
- **S1, plafond de ~11 appels par pas, et le pas s'arrête au lieu de s'enchaîner** : `tours[23]` de chaque run de D6 : « This is your last step: no more tool calls are possible. Answer now… » ; puis la copie de l'hôte et plus rien pendant 6 min (`run: None`, `running: true`). Pour lire, éditer et jouer deux cargo test, le budget est épuisé par la lecture à chaque pas.
- **S2, l'agent repart de zéro à chaque pas** : le contexte du pas N+1 dit « tu ne vois plus leurs appels d'outils : relis un fichier s'il te le faut ». D4 relit le même read_file ; D5 refait toute la recherche ; D6 relit backend.rs 990–1040 et 9030–9090 à chacun des 7 pas.
- **S3, une nouvelle `request` remplace la demande en cours** : au pas 2 de « continue jusqu'au bout », `tours[0]` ne porte que la consigne, sans « La demande en cours : … » (une `answer` la garde, D2 pas 2). L'agent a cherché le cahier des charges dans les docs du dépôt (la page de vision, lue 5 fois) et travaillé sur la fiche de c7 plutôt que sur la demande.
- **S4, `step_done` avant `report`** : D2, D4, D5 : il clôt le pas en disant « fait : identifié » sans avoir écrit le contenu ; l'hôte relance et un second pas complet suit (3 cas sur 4 demandes de lecture, 12 à 50 s chacun).
- **S5, présentation répétée** : « Je suis Algareth. » en tête de D1, D3, D6 ×4 ; l'invite (chat.rs:709) dit « In the first answer of a task, introduce yourself… once » et l'hôte traite chaque run comme une première réponse.
- **S6, curiosité sur soi** : grep « Algareth », « Your name is Algareth », lecture de chat.rs 690–729 et de teammate_tests.rs (D1), grep « Algareth » 53 résultats (D5), grep « /nom », « agent: », `git show` du commit de c7 (D6) : ces lectures mangent le budget S1.
- **J1, `demande` figée** dans le journal des runs.

**Primitives (ED)**
- **P1, `search_code` jamais choisi** (0 appel sur 82 en D6, 0 en D5) alors que la description de grep_files dit « pour une question de sens, search_code fait mieux » ; `usages` une fois (juste), `outline` une fois après le grep. Les primitives elles-mêmes ont toutes répondu juste (usages exact et classé, read_file au bit, outline complet, grep_files exact avec « 48 skipped: too large » et borne à 50).
- **P2, `grep_files` sans garde** : `agent`, `manifest`, `/nom` rendent 4 800 à 5 100 résultats (12 Kio de contexte) sans refus ni proposition de `path_prefix` avant d'avoir rendu les 50 premiers.

**Porte (ED)**
- **G1, `git rev-parse` classé « écrit »** : commande.rs:118 `LECTURE_SEULE` ne liste que status/diff/log/show/branch/remote/blame ; « dans le doute, ça écrit » → accord demandé pour une lecture pure. À ajouter : rev-parse, ls-files, describe.

**Index** : rien de rouge ; usages et outline disent « indexé il y a 12 min, à jour » ; lignes conformes à master (vérifié par md5 et diff).

**Terminal (tty)**
- **T1, `--plain` rend la main avant la fin quand il démarre l'équipier** : « demande posée (l'équipier part) » puis `fin=0` en 3 s, journal réduit à la demande ; quand l'équipier tournait déjà il reste attaché jusqu'à « fini ». En mode script, seuls la boîte et le journal des runs font foi.

État laissé : tâche c7-essai `running: true`, aucun run, boîte à 41 lignes, arbre propre ; tâches de Lucie intactes ; aucun processus tué. Fichiers de la campagne dans le bac de c7 (`algareth/d1..d6d.*`, `dump_run.py`, `lancer.sh`).
