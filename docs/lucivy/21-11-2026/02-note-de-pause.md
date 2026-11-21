# Note de pause, session lucivy, 21 novembre 2026, 15 h 26

Redémarrage de la tablette par Lucie (sortir /tmp de la RAM). À reprendre par `claude --resume`.

## Où j'en suis

- **lucivy 4.3.13 et 4.4.0 publiées** (tags `v4.3.13` le 20 à 21 h 27, `v4.4.0` le 21 à 07 h 25, au mot de Lucie), leurs
  pointeurs atterris dans rag3weaver (2bfcc013d2, 5df6f827fa).
- **Le lot (A)**, les générations du plein texte par paquet (décision de Lucie, 21 nov. 08 h 2x) : codé, vérifié,
  **non déposé**. La mesure en debug donne une remise en état du plein texte de 2 174 → 260 ms après un arrêt, mais
  une reprise entière de 145 → 172 s (+18 %). c7 a choisi la voie (b) : mesurer en release, puis décider.
- **Faits ce jour et atterris** : la sonde de 05 h 00 verte (0 perdu sur 14 index), le ticket des mots perdus fermé ; la
  fausse alarme « écritures perdues » corrigée (PR 359) ; la PR #60 de lucivy (les sauts dits en clair).

## Ce qui tourne à luciepc

- **La mesure en release** : `poste cpu timeout 360m bash ~/.cache/rag3weaver-build/generations-par-paquet/release.sh`,
  pids 3249702 (poste), 4019632 (timeout), 4019634 (bash), lancée à 15 h 08. Elle bâtit en release les worktrees
  `lucivy-session/21-11-2026-generations-avant` (ae9f53d024 et les fichiers de test du mode mesure, copiés sans commit)
  et `lucivy-session/21-11-2026-generations` (4f276d6f2d), dans `target-lucivy-release-<worktree>`, puis chronomètre
  trois passes ordinaires de chaque côté (rôle témoin, 100 000 scopes, sans mort). Journal :
  `~/.cache/rag3weaver-build/generations-par-paquet/release.out` (lignes `PASSE <worktree> <répétition> <ms>`, puis
  `FIN`).

## Mes branches

- `generations-par-paquet-2` (rag3db) : **4f276d6f2d**, le lot (A). Attestation VERT 8/8 sur cette tête (luciepc,
  lib 424ce93e56). Témoins : `tests/e2e_generations_*`. Ne pas la changer avant la décision (consigne de c7).
- `lucivy-page-place-des-generations` (docs, déposée en `lucivy-page-place-des-generations-2`) : la page 01 (la place
  d'une génération gardée, les lucids, la compaction par étapes).
- `lucivy-ticket-close-sans-generation` (docs, déposé) : le ticket de lucivy 4.4.0, la fermeture perd sa génération.
- lucivy : `main` porte 4.4.0 ; aucune branche ouverte.

## Ce qui attend un dépôt ou une décision

1. **(A)**, selon la mesure en release :
   - un écart dans le bruit : déposer `generations-par-paquet-2` tel quel (rust, témoin
     `a_kill_two_generations_ahead_comes_back_without_a_rebuild`, suite en plus e2e_tx_par_paquet_arret), plus un
     ticket pour le reste du coût ;
   - au-dessus de 5 % : préparer une lucivy 4.4.1 qui étiquette le dernier commit sans en refaire un, avec son témoin,
     mesurer (A) dessus sans la publier, et c7 porte la publication à Lucie.
2. **La 4.4.1** doit aussi régler le ticket de la fermeture sans génération.
3. **La compaction par shard** (`compact_shard(i)`, Lucie : « oui complètement ») : un lot après (A).

## Ma prochaine action

Lire `release.out` à luciepc. Si `FIN` y est, comparer les trois passes de chaque côté et appliquer la règle de c7
ci-dessus. Sinon, attendre (`poste --qui`). Après la décision : retirer les worktrees `21-11-2026-generations-avant` des
deux postes, et les targets `target-lucivy-release-*` de luciepc.
