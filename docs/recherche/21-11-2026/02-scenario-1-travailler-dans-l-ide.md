# Scénario 1 : un développeur qui connaît les lacunes travaille dans l'IDE

Session recherche, nuit du 20 au 21 novembre 2026, sur la consigne de Lucie : l'IDE est un environnement de travail,
plus complet que le terminal, avec tous les agents actifs ou non, tous les dossiers du poste ouverts dans rag3weaver.
Le scénario juge le travail : les agents et leurs tâches, les dossiers ouverts, piloter un agent du terminal depuis
l'IDE et l'inverse, répondre aux accords, suivre une tâche vivante, la recherche comme un outil parmi d'autres.

## Le montage

- L'IDE de l'hôte de service de la tablette, entré par son URL d'entrée, dans un Chrome sans tête piloté par CDP : le
  chemin de Lucie.
- Rien écrit dans `rag3db` ni dans les bases de Lucie : sur `rag3db`, des questions en lecture seule. L'aller-retour
  avec le terminal se joue sur `projet-tty` (76ad218634aa), une copie du crate du terminal préparée par tty.

## L'hôte

| Heure | Ce qui se passe |
|---|---|
| 01 h 29 | Hôte du soir posé (fed132c964). Le chat de `rag3db` reste sur « Connexion… » ; `/ide/chat` et `search_code` (MCP, toutes les sessions) : « il faut la feature token-count ». Le binaire contient ce message, compilé seulement sans `token-count` : bâti sans les features de service. Dit à c7 et G. |
| 01 h 33 | Retour à l'ancien hôte (83c667e420). |
| 01 h 55 | Hôte rebâti posé (token-count présent). `rag3db` se rouvre jusqu'à 02 h 13 : chat et outils refusent pendant ce temps (ticket `2026-11-21-un-gros-depot-se-rouvre-dix-huit-minutes-apres-une-pose`). |

Le défaut du bâti a déjà son ticket (`everything-declarative/tickets/2026-11-21-un-bati-sans-token-count-refuse-d-ouvrir.md`).
Reste à la procédure de pose : contrôler le binaire avant de le poser.

## Le parcours

| Ce que la consigne demande | Ce que l'IDE fait | Verdict |
|---|---|---|
| Les agents, actifs ou non, et leurs tâches | Les sessions de chat par dépôt et leurs tâches (finie, en attente, fermées avec leur motif). Les agents Claude du poste n'y sont pas ; `/ide/etat` rend `sessions: []`. | **rouge** : ticket `2026-11-21-l-ide-ne-montre-pas-les-agents-du-poste` |
| Les dossiers du poste ouverts dans rag3weaver | Deux dépôts à l'accueil ; `projet-tty` seulement sous « Disponibles » ; ni les worktrees des sessions ni les vues. | **rouge** : ticket `2026-11-21-l-ide-ne-montre-pas-les-dossiers-du-poste` ; « Ouvrir un dossier » arrive (lot `recherche-ide-ouvrir`) |
| Ouvrir un dossier et y travailler | Pas sur l'hôte de service (le lot n'a pas atterri) ; vert au témoin navigateur : créé et `git init`, ou attaché, ou refusé en le disant. | à rejouer après l'atterrissage |
| Piloter depuis l'IDE un agent lancé au terminal | Tâche `essai-scenario-1` de tty, page vivante. Mon tour par « écrire à l'équipier » (02:03:11 `ide · request`) paraît au terminal (`[ide · demande]`) ; la réponse arrive dans la page à 02:12:34. | **vert** (neuf minutes : un `cargo check` bloqué avant, non tranché, relevé de tty) |
| L'inverse : suivre depuis l'IDE ce que le terminal fait | Les consignes de tty (`supervisor · instruction`), les pas et les reports de l'équipier paraissent dans la page sans la recharger. | **vert** |
| Répondre aux accords | `python3 -c "print(42)"` (hors de la porte de la tâche) : carte « L'équipier attend ton accord », « Oui » depuis la page (02:18:13 `ide · answer oui`) ; `› accordé` au terminal ; la commande tourne, l'équipier rend `42`. | **vert** |
| La recherche comme un outil | Chat de `rag3db · master` (gemini-3.8-flash), une question en lecture seule : cinq outils (`grep_files`, `read_file`) en 7 s, la bonne réponse (le refus d'`open_view`, sa ligne, son texte). | **vert** |

À côté : les pages des tâches parlent encore de « backends » (ticket `2026-11-21-les-pages-des-taches-parlent-de-backends`) ;
le chat répète à chaque ouverture ses sessions orphelines (ticket `2026-11-21-le-chat-montre-a-chaque-ouverture-les-sessions-orphelines`).

## Verdict

**Rouge sur la vue d'ensemble, vert sur le travail.** Piloter une tâche, l'accorder, la suivre et chercher marchent
depuis l'IDE. Ce qui manque est ce que Lucie a mis en premier : voir tous les agents et tous les dossiers du poste.

## Pour la page « parcours d'essai » (tty)

- Après une pose, un gros dépôt peut refuser chat et outils de longues minutes : relire `daemon_status` avant de
  conclure.
- Les agents du poste ne sont pas dans l'IDE : `ListAgents` côté Claude Code.
- Une tâche lancée au terminal se pilote par sa page `/ide/code/<dépôt>/taches/<nom>` : « écrire à l'équipier », et
  la carte `#attente-equipier` pour les accords.
