# Rapport de session — 1er octobre 2026, soir

Écrit par la session orchestratrice juste avant une compression de son
contexte. Il dit ce qui a été fait ce soir, ce qui est **en vol** et à
contrôler, ce que Lucie a tranché, et ce qui l'attend. Le registre vivant
reste [`docs/journal-des-chantiers.md`](../../../../docs/journal-des-chantiers.md).

## 1. Ce qui a été fait

| Quoi | Où |
|---|---|
| Remise à niveau sur les changements du 19 au 27 septembre (Codex, puis la session Products Experiments) | docs du 25 et du 27 septembre, relus |
| Le **journal des chantiers** créé | `docs/journal-des-chantiers.md` |
| `mtg-experiments` poussée : 14 commits n'existaient que sur le poste | — |
| Historique de `mtg-experiments` **réécrit** pour retirer 17 trailers d'attribution, poussé en force | hash changés à partir de `64a12b05b` |
| Réconciliation des objectifs, knowledge dump, deux relevés des docs de vision contre le code | ce dossier, `01` à `04` |
| Correctifs du moteur C++ extraits dans un commit à part, avec le test qui manquait | `9edf6f3b4` |
| **`mtg-experiments` fusionnée dans `master`**, vérifiée, poussée | fusion `32d6e0b44`, master `f09d51d6a` |
| Trois rouges remontés par la fusion, corrigés | `d83a557d3`, `f15fab787`, `d4f32f9e0` |
| Cause du **WAL illisible** trouvée | §3 |

Master est vert au 1er octobre : 1 103 unitaires ; `e2e_search` 40,
`e2e_simple_entity` 22, `e2e_generic_search` 18,
`e2e_idempotent_registration` 22, `e2e_native` 11, `e2e_entites_derivees` 3,
`e2e_chemin_de_masse` 4, `e2e_code` 24, `e2e_undo` 4, `e2e_prise_atomique` 12,
`e2e_phase0b` 14, `e2e_catalogue_gabarits` 12, `structured_payloads` 3, et
les quatre tests Python du backend. Non joués sur cette machine : les suites
`e2e_burn_*`, les bancs, `e2e_postgres` (pas de serveur).

## 2. Ce que la fusion a appris

- **Une branche longue cache ses rouges.** `e2e_search` portait depuis onze
  jours deux tests rouges que personne n'avait joués sur la branche.
- **`Catalog::get` rend une ligne à plat** depuis `ab95c3a2d` (20 septembre) :
  plus de clé `n`. L'ancienne forme était une fuite du `RETURN n` de Cypher ;
  la nouvelle est la même sur les deux dialectes. Le contrat est maintenant
  écrit et testé (`e2e_search::get_rend_une_ligne_a_plat`).
- **Le chat n'envoie `journal` qu'à un backend qui l'annonce** : `describe`
  porte `capabilities`.
- **16 tests ne tournaient pas** faute des poids MiniLM sur la nouvelle
  machine ; ils sont installés. « Non joué » n'est jamais « vert ».
- **Le backend déclaratif ne contourne pas le moteur** : ses outils
  d'ingestion passent par `Catalog::ingest_entities`. Ce qui manque est
  ailleurs (§5).

## 3. Le WAL illisible : la cause

Ce n'était pas l'arrêt brutal. **`ChecksumWriter` corrompt à l'écriture tout
enregistrement de plus de 4 096 octets** : `resizeBufferIfNeeded`
(`src/storage/wal/checksum_writer.cpp`) remplace le tampon par un neuf non
initialisé sans recopier ce qui y était. L'enregistrement perd son début,
octet de type compris ; la somme de contrôle, calculée sur le tampon faux,
est valide et ne détecte rien. Le lecteur (`checksum_reader.cpp`) a le même
défaut. Un arrêt propre fait un checkpoint et supprime le journal, donc on ne
le voit jamais ; un arrêt brutal force le rejeu d'un journal déjà faux. Le
bug vient de l'amont (« Checksum WAL records », #5940) ; ni `vela/master` ni
`vela/storage/concurrent-checkpoint-recovery` ne le corrigent.

Preuves : les quatre journaux mis de côté le 27 septembre se découpent
entièrement, finissent sur un COMMIT, et tous leurs enregistrements
illisibles dépassent 4 096 octets.

**La base MTG actuelle a un journal déjà corrompu** (30 enregistrements) :
ne pas l'ouvrir avec `throw_on_wal_replay_failure=false`, le rejeu
tronquerait le journal. Ce qui a été écrit depuis son dernier checkpoint est
perdu ; elle était à reconstruire de toute façon.

## 4. Ce qui est en vol — à contrôler au retour

La session « rag3db Products Experiments » exécute ; l'orchestratrice
contrôle par git à chaque étape. Branche `correctif-wal-enregistrements-longs`,
depuis `f09d51d6a`.

**Étape D — le bug d'écriture.** Consigne donnée :
1. le test d'abord, dans `test/transaction/wal_test.cpp` : chaînes de 4 097,
   8 193 et ~100 000 caractères, plusieurs gros enregistrements dans une
   transaction, `force_checkpoint_on_close=false`, réouverture avec
   `throwOnWalReplayFailure=true`, égalité des **valeurs** relues ; prouvé
   rouge sur master ;
2. recopier l'existant dans `resizeBufferIfNeeded`, écrivain **et** lecteur ;
   remplacer `default: KU_UNREACHABLE` de `wal_record.cpp` par une exception
   « journal corrompu » ; format du journal inchangé ;
3. le rapport d'analyse versionné comme doc du correctif (`docs/` racine) ;
4. **arrêt et compte rendu avant de reconstruire `build/lecteurs-csv`**.

**Étape E — la fin déchirée**, dans un commit à part. Décision de Lucie
(« oui pour option 1 ») : une fin de journal déchirée n'empêche plus
d'ouvrir ; on rejoue jusqu'au dernier COMMIT complet. Contour fixé :
- une **corruption ailleurs qu'en fin de fichier reste un refus d'ouvrir** ;
- les octets écartés sont **copiés dans un fichier à côté** avant la
  troncature, et dits sur stderr ;
- les deux lectures courtes de `BufferedFileReader` sont corrigées au passage ;
- tests : journal tronqué à plusieurs longueurs avec les réglages par défaut,
  et une corruption au milieu qui doit refuser.

**Ce qu'il faut vérifier au compte rendu** : le test est-il prouvé rouge
avant le correctif ; le diff ne touche-t-il que `src/storage/wal/`,
`src/common/serializer/` et les tests ; les tests existants qui tronquent de
10 octets disent-ils encore vrai sans avoir été relâchés. Puis : reconstruire
`build/lecteurs-csv`, rejouer `e2e_prise_atomique`, `e2e_checkpoint`,
`e2e_undo`, `e2e_search`, `e2e_chemin_de_masse` et les tests de persistance
du backend, fusionner en local, vérifier, pousser.

**Une question ouverte par l'analyse, non vérifiée** : un lecteur qui ouvre
la base pendant qu'un écrivain a un journal non vide rejoue-t-il ce journal ?
Si oui, il tombait sur le même défaut.

## 5. Ce que Lucie a tranché ce soir

- **MTG : on pousse tout sauf les données.** Le code des branchements a sa
  place dans git même s'il n'est branché que sur le poste.
- **Retirer les mentions d'IA de l'historique** : fait sur `mtg-experiments`.
  Quatre commits anciens de `master` en portent encore (un de février 2026,
  trois de l'amont Kuzu) et ne sont **pas** réécrits.
- **Un journal des chantiers** entre sessions.
- **Go sur les points 1 et 2** de la réconciliation ; l'autre session
  exécute, celle-ci contrôle.
- **Fin de journal déchirée : on ouvre** (étape E).
- **Remettre l'avertissement « 0 relue »** de `split_unchanged` hors du
  chemin de masse : fait (`4b5f63295`).
- **Le harnais reste tel quel** : il rend déjà toutes les erreurs d'une étape
  d'un coup ; les seules frontières sont schéma → `before` → `after`.
- Deux chantiers inscrits au journal, après D et E : **supprimer les lignes
  disparues à la synchronisation** (générique, dans le moteur) ; **lever le
  refus d'écrire en lot sur une entité à `Lifecycle`**.

## 6. Ce qui attend Lucie

Les six décisions du pas C, avec ce que je recommande, pour qu'un « oui »
suffise :

| # | Décision | Recommandation |
|---|---|---|
| 1 | La forme du pas C | Oui : poids de fusion par entité, pondération par valeur de champ (le genre dans `Scope`), gabarits de dérivées. Un poids, pas un filtre. |
| 2 | Poids de fusion par défaut, 0,6 / 0,4 ou 0,3 / 0,7 | **Mesurer avant de trancher** : rejouer `e2e_banc_etage` en hybride aux deux réglages. Le banc existe pour ça. |
| 3 | Embarquer le nom avec le corps | Plus tard : +0,06 de MRR, mais ça touche les décalages et les surlignages. |
| 4 | `fuse_results` à trois listes, orpheline | Supprimer avec ses tests. |
| 5 | La grappe d'exploration après recherche | Supprimer : le champ `graph` est toujours vide, et le verbe `follow` fait maintenant ce parcours. |
| 6 | `search_with_strategy`, sans appelant de production | Supprimer. |

Et aussi : le ménage (trois worktrees morts, neuf branches fusionnées, cinq
fichiers non suivis à la racine) ; les sept PR amont burn / cubek ; le budget
de reprise du lecteur ; le design des accès déclaratifs.

## 7. La suite proposée

Après D et E, dans l'ordre de la réconciliation, avec les réserves de
l'autre session :

1. la place disque : récupérer les lignes supprimées dans rag3db (même cœur,
   même gravité que le journal) ;
2. le pas C, après les décisions ci-dessus ;
3. la description d'entité dans le manifeste, reprise par les descriptions
   d'outils (une journée, générique) ;
4. **l'agent de code servi par le backend déclaratif**, avec l'entrée
   « indexer ce dépôt » : c'est le deuxième vrai cas d'usage du backend, donc
   le test de sa généricité ;
5. un backend composé par un agent à partir de gabarits.

Les deux petits chantiers du §5 et les restes du deck builder se glissent
entre deux pas.
