# Orchestration — rapport de session du 11 novembre 2026 (16 h 30)

Session « orchestration sessions » (c7). Ce rapport dit l'état du projet pour un agent qui reprend
l'orchestration sans avoir lu nos échanges. Le knowledge dump (`02-knowledge-dump.md`) dit les règles.

## Ce qui se passe ce 11 novembre

Lucie passe ses sessions sur Codex pour garder du crédit pour son travail. Chaque session écrit son
rapport et son knowledge dump sous `docs/<session>/11-11-2026/`, et ses tickets ouverts iront dans
`docs/<session>/tickets/` dès que `check_tickets.py` saura les lire (lot demandé à la session
coeur-cpp-hotfixs-tickets ; le mot « déplace » part à son hash). Les visions sont rassemblées sous
`docs/orchestration/visions/`, les nouveautés résumées sous `docs/orchestration/news/`.

## L'état des postes (dernier relevé : 9 novembre, 13 h 42)

- **Lib du moteur posée aux deux postes** : `rag3db-build-29a7bc52aaf11e6204f9beafaffb154105417e13-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF`
  (md5 b65cc5be), le correctif des deux sens des petites tables de relations (29a7bc52a). Au magasin
  aussi : 9bae60786 (gardes de l'étendue) et cbc06c3e1. Toute passe épingle cette lib par son
  identifiant complet, sans `RAG3WEAVER_MOTEUR_ANCIEN`.
- **Binaire rag3weaver posé** : 0b0ff221a (tête de master du 9 à 13 h 38), luciepc à 13 h 39, portable
  à 13 h 41 ; depuis, master a reçu a6637eeee, be319fa75, 3e8c87aa5, ab8272deb (archi, 96) : **une pose
  du binaire est due** (lib inchangée). Session optimiseur : `preparer-binaire.sh <hash>` puis
  `poser-binaire.sh <hash> luciepc` puis `portable`, préavis d'une minute, l'atelier de Lucie
  rouvert avec ses 11 outils.
- **Les bases de Lucie gardent leur ancien écart des deux sens** (INHERITS_FROM 650/658, LOCKS
  544/546) : le correctif empêche d'en créer, il ne répare pas. Réparer = un index complet de ses
  bases, à son mot (sa carte, son heure).
- **luciepc** : `discard=async` retiré, `fstrim.timer` hebdomadaire ; Lucie doit poser le drop-in
  `sudo systemctl edit fstrim.service` avec `ExecStart=/usr/bin/fstrim --listed-in /etc/fstab:/proc/self/mountinfo --verbose --quiet-unsupported --minimum 16M`
  et une heure creuse, avant le lundi 16 novembre 00 h 02. Disque à ~65 %. Plafond `lourds-max` : 4.
- **Services d'embarquement à luciepc** (card2) : 7878 granite-278m, 7879 bge-m3, 7880 granite-107m,
  7882 granite privé (hotfixs). Tunnels depuis le portable : 7979 → 7878, 7980 → 7879, 7981 → 7880.
  **Jamais un cinquième service sur card2** (82 % de VRAM : le cinquième a mis la carte en reset de
  ring le 9). card0 est celle de Lucie quand elle joue : à son mot seulement.

## La file des pushes au 9 novembre, 14 h 40 (ce qui a atterri ce jour-là)

29a7bc52a (cœur, deux sens) · 833c89539 (tickets du cœur) · 120a5db31 (hooks-essai, le rappel dit ce
qu'il attendait) · 3b119beb6 (96, fiches classées, allow_create éteint) · 92180836b (recherche,
résidus de suite) · a9872fb87 (96, bancs hors de l'e2e entier) · 6a4d55d13 (96, mesure de l'équipier,
trois tickets) · 8efd3eeb0 (archi, délai des témoins de l'hôte) · d23af9cd2 (archi, l'atelier lit et
écrit) · 0b0ff221a (96, dossier d'état des témoins) · a6637eeee (96, recul du modèle borné) ·
be319fa75 (archi, commandes en attente sur la carte, famille de commandes corrigée) · 3e8c87aa5
(archi, carte de chemin relance) · ab8272deb (archi, jauge du contexte).

## Ce qui restait ouvert, session par session (à relire dans leurs rapports du 11)

- **everything declarative** : le correctif des réclamations du rattrapage (témoin 219053a4b ; la CI de
  master rougit par intermittence sur `owed_vectors_are_caught_up_on_open_when_the_manifest_declares_it`
  tant qu'il n'est pas atterri) ; la relecture des pages (vues par worktree, l'atelier lit et écrit,
  M8) ; le ticket « une tâche se dit faite sans livrable » (règle de teammate.rs) ; les défauts de
  l'installeur de l'atelier ; le lot « commit des gabarits livrés ».
- **recherche** : arret-du-backend (branche arret-du-backend-6, passe lancée le 9 à 13 h 24, verdict non
  reçu) ; le journal des runs écrit après l'arrêt de l'hôte (ticket, lot suivant) ; le Surveillant ;
  lucivy PR #22 et #23 (fusion au mot de Lucie dans la session lucivy).
- **rag3weaver archi** : `ide-par-le-flux` (wt-flux, a9c5a3108, passe tuée code 137 : à rejouer) ; le
  découpage de ce qui pèse au survol de la jauge ; les propositions et le fichier ouvert par le flux.
- **mémoire (96)** : l'archivage à blanc des docs sur une copie de la base de Lucie (bloqué par un 400
  INVALID_ARGUMENT de Vertex muet : ticket et petit lot « le client garde la requête d'un 4xx ») ; M8
  après relecture d'ED ; le classeur local après le mot de Lucie (la mesure dit que le contenu manque,
  pas le juge : 92 % des rappels vides).
- **codeparsers** : require-js (mesure finie le 9 à 02 h 31, lecture jamais rendue : session bloquée sur
  une approbation) ; ticket import_binding.
- **coeur c++** : l'annulation inachevée après un pool plein (témoin réduit) ; fermer le ticket de
  l'index complet quand hotfixs a rejoué sa trace sous 29a7bc52a.
- **coeur c++/hotfixs/tickets** : le rejeu de la trace exacte sous 29a7bc52a (7882/7992) ; le script
  des tickets pour `docs/*/tickets/`.
- **banc** : le banc Qwen3-4B en GGUF par llama-server (bâtir llama.cpp à luciepc depuis les sources
  épinglées, ou paquet pacman par Lucie : **à trancher par Lucie**, mon mot ayant été refusé par le
  classificateur).
- **optimiseur** : la pose du binaire due ; la nuance de la garde du réveil (un service lent n'est pas
  absent).
- **hooks-essai** (session fermée) : page « la recherche de code par les hooks » en attente du mot de
  Lucie (`extension/rag3weaver/docs/07-novembre-2026-hooks-essai/04-…`).

## Pour Lucie (sa main)

- Trancher la voie de banc (llama.cpp bâti ou paquet) ; lire les pages des vues, de l'atelier, des
  hooks, du classeur local ; le drop-in fstrim ; les hooks dans toutes les sessions et la ligne CLAUDE.md
  « search_code/usages d'abord » ; le mot pour les fusions lucivy ; l'index complet de ses bases ; le
  ménage des 476 `livre.json` de témoins listés dans le ticket de 96 ; la suppression des branches
  distantes fusionnées (jamais sans elle).

## Les trois prochains pas de l'orchestration

1. Recevoir les rapports et dumps des treize sessions, puis « déplace » pour les tickets au hash du
   script ; relire `docs/README.md`.
2. Faire poser le binaire de la tête de master (optimiseur), une seule coupure.
3. Remettre la file à l'agent suivant : d'abord ED (réclamations, CI), puis recherche
   (arret-du-backend), puis archi (ide-par-le-flux).
