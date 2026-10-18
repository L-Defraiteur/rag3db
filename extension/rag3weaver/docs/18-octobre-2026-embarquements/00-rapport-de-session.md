# Session embarquements (chantier F) — rapport du 18 octobre 2026, reprise après la pause

*Suite du rapport du 11-12 octobre (`11-octobre-2026-embarquements/00-rapport-de-session.md`).
Pause de Lucie du 12 au 17 octobre ; aucun lot n'attendait : tout était sur master
(`ec1707ee2`, le retrait du saut en deux temps et le correctif PostgreSQL du saut sans
table d'arrivée).*

## La latence de `POST /embed` sur les trois services de luciepc

Demandée par l'arbre principal ; jouée le 18 octobre de 14 h 34 à 14 h 37, seule sur
luciepc (jeton de l'orchestration, aucun bâti ni index en cours, l'hôte relancé à 14 h 32
sans travail). Client : `~/.cache/rag3weaver-build/embarquements/latence-embed.py`, par
le tunnel `127.0.0.1:7979-7981`. Pour chaque service, `GET /sante`, puis deux régimes :

- **doux** : les morceaux qui tiennent dans 4 096 caractères (le budget
  `RAG3WEAVER_EMBED_CHAR_BUDGET` du régime doux), soit 3 morceaux ;
- **plein** : `lot_conseille[0]` morceaux.

Les morceaux sont de vrais extraits de `src/`, 1 200 caractères au plus (la taille d'un
morceau de `Scope`). Cinq appels de chauffe, puis 50 mesurés en série ; à 50 appels, le
p99 est le max.

| modèle (port, `lot_conseille`) | doux : médiane / max | plein : médiane / max | plein : débit |
|---|---|---|---|
| granite-278m (7979, 128×512) | 36,2 / 89,2 ms | 1 229,5 / 1 484,7 ms (128 morceaux) | 104 morceaux/s |
| bge-m3 (7980, 32×512, dual) | 69,9 / 80,3 ms | 829,8 / 1 860,5 ms (32 morceaux) | 39 morceaux/s |
| granite-107m (7981, 128×512) | 22,4 / 133,0 ms | 541,0 / 583,9 ms (128 morceaux) | 237 morceaux/s |

**La charge, pour prouver la mesure seule** (`nvidia-smi` absent : la carte est AMD, son
occupation se lit dans `/sys/class/drm/card*/device/gpu_busy_percent`) :

- avant, 14 h 34 : charge moyenne 4,87 / 6,47 / 6,74, venue du bureau et de la session de
  jeu (`seat1-input-monitor`, `plasmashell`, `wireplumber game-hdmi`), pas d'un bâti ;
  card2 (Navi 48, celle des services, 25,7 Go de VRAM occupés) à 15-17 % de fond sur dix
  échantillons, card0 à 0 % ;
- après, 14 h 37 : charge moyenne 3,75 / 5,23 / 6,21 ; card2 à 65 % au premier
  échantillon (la queue de la mesure), puis 16-17 % ; card0 à 0 %.

**Lecture** : le régime doux tient sous 100 ms en médiane pour les trois modèles ; ses
queues (89 et 133 ms) sont des pointes isolées sur 50 appels. Le plein sature la carte
mais une instruction y dure de 0,5 à 1,9 s.

## Fermeture de la session

- Effacés : les cinq reflinks de mesure et quatre libs copiées (environ 10 Go), puis
  `target-F` (9,4 Go) ; à la fermeture, `wt-docs-F` et le worktree
  `~/git_workspaces/rag3db-embarquements` ; `target-embarquements` sur luciepc l'était
  depuis le 12 octobre.
- Gardés sous `~/.cache/rag3weaver-build/embarquements/` : `copie-depot-scellee`, la
  preuve du P1 (lecture seule, sha256 `551a178b121aafa1…`), tant que le banc n'a pas
  reproduit ; `copie-depot` ; `lib-pas3` (la lib du troisième pas du cœur,
  `13f4647dc`) ; les scripts et leurs sorties.
- Rien d'ouvert à moi : pas de ticket à fermer, pas de branche distante.
