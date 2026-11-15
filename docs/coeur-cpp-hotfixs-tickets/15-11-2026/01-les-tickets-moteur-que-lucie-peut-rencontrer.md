# Les tickets moteur que Lucie peut rencontrer sur ses bases aujourd'hui

15 novembre 2026, hotfixs, à la question de Lucie relayée par c7 : parmi les tickets moteur ouverts, lesquels sont
des défauts qu'elle peut rencontrer sur ses bases **aujourd'hui** (perte, plantage, réponse fausse atteignables en
service), par opposition aux cas de charge du banc ?

**Aujourd'hui**, cela veut dire :
- la lib 373b5477b, posée aux deux postes le 14 au soir, avec le binaire fac67a208 (luciepc) puis 26a091474
  (portable) ;
- un hôte par poste et quelques sessions ;
- des index vectoriels remplis par `embed_set` et le plein texte en fichiers ;
- ni vingt écrivains concurrents, ni pool saturé, ni extension geo.

Le relevé a été lu le 15 novembre à 13 h : 54 tickets ouverts de famille moteur, toutes sessions confondues.

## 1. À fermer : corrigés dans la lib posée (3)

Leur correctif est un ancêtre de 373b5477b, et cette lib est posée aux deux postes. Chacun attendait « la lib
posée » pour se fermer.

| ticket | ce que c'était | le correctif, dans 373b5477b |
|---|---|---|
| `coeur-cpp/…/2026-11-13-un-commit-sous-index-vectoriel-saute-des-tables-de-noeuds` | **perte** : un COMMIT réussi ne publiait pas certaines tables de nœuds | 8275be382 (+ la garde e93710f36) |
| `coeur-cpp/…/2026-11-13-une-recherche-par-cle-primaire-rate-des-cles-validees-pendant-un-point-de-reprise` | **lecture fausse** : une clé validée dite absente | 0c41f9fe6 |
| `coeur-cpp/…/2026-11-14-sous-vingt-ecrivains-et-un-pool-plein-des-note-validees-manquent` | **perte** de lignes validées sous pool plein | 373b5477b |

## 2. Atteignables aujourd'hui, par gravité

### Plantage

1. **`2026-11-08-un-sigsegv-dans-optimisticread-sous-lecture-concurrente-pendant-une-indexation`.** Le seul
   plantage **vu en service** : l'hôte de Lucie, le 8 novembre à 8 h 50, relancé par systemd. Un mécanisme
   est prouvé, mais pas le premier événement. C'est le premier à finir.
2. **`2026-11-08-le-create-vector-index-concurrent-lit-une-ligne-au-dela-de-la-colonne-en-cache`.** Une
   lecture hors allocation vue sous ASan, dès qu'un `CREATE_VECTOR_INDEX` tourne sous des écritures de sa
   table. rag3weaver lance ses CREATE sous `Mutex<Catalog>`, mais lucivy vient de montrer que le DAG d'un
   `/call` écrit hors de ce verrou. Ce plantage reste donc atteignable, rarement.
3. **Le dictionnaire et la colonne ajoutée sous écrivains.** Deux tickets : `2026-11-14-un-point-de-reprise-automatique-meurt-dans-le-dictionnaire…`
   et `2026-11-15-ddl-every-minute-meurt-d-un-sigsegv…`. Il faut un ALTER sous des écrivains, et l'hôte n'en
   fait qu'à l'ouverture ou à une montée de schéma (la M8 de ce matin en était une). Atteignable mais rare ;
   le lot du verrou de schéma du cœur ferme la famille.

### Perte

4. **`2026-10-04-durabilite-sur-faute-d-entree-sortie-ou-coupure`** : un disque plein ou une coupure au
   mauvais instant. Atteignable par nature. Deux causes sont corrigées avec témoin ; le reste est en cours.
5. **`2026-10-10-ouverture-sans-une-extension-dont-une-table-depend`** : un index perdu sans que l'ouverture
   le dise, si le fichier d'une extension manque au rejeu. C'est une donnée dérivée, à rebâtir. Les bases
   portent dans leur `.extensions` le chemin de l'extension de la lib d'avant (avertissement « extension
   VECTOR could not be loaded » vu cette nuit sur toutes les copies) : le cas n'est pas théorique.

### Réponse fausse

6. **`2026-11-14-les-deux-index-d-une-table-de-relations-ne-s-accordent-plus`** : à **dégrader**. La cause
   (l'annulation d'un COMMIT refusé) est corrigée dans 373b5477b. Les deux bases touchées sont réparées
   cette nuit. Le portable est sain au relevé ancré. Reste la perte « des deux côtés », localisée par le cœur
   mais non expliquée, et couverte par 373b5477b et son témoin. Je propose de le garder ouvert, gravité
   « donnée fausse, existant réparé, cause couverte ».
7. **`2026-11-07-un-rang-deplace-pendant-un-create-n-est-pas-toujours-rattrape-premier`** : à la marge, un
   rang mis à jour pendant un CREATE est trouvé, mais pas premier.
8. **`2026-10-10-index-plein-texte-en-fichiers-hors-du-point-de-reprise`** : une réponse incomplète après un
   arrêt brutal, toujours dite par l'état d'index. C'est la décision de Lucie (« comme Neo4j »).
9. Les orphelins du plein texte (ticket de **recherche**, famille produit, cité pour mémoire) : purgés cette
   nuit sur les trois bases servies, mais toujours produits en service (970 de plus en douze heures au
   dépôt). C'est la vraie « réponse fausse » courante de Lucie : des orphelins prenaient jusqu'à deux
   résultats sur trois. Le correctif de `DeleteRecordNode` chez recherche passe avant tout le reste de cette
   liste.

### Gêne en service, sans perte (pour mémoire, pas des défauts de données)

- **Blocages et attentes** :
  - `un-point-de-reprise-fige-toutes-les-transactions` ;
  - `sur-une-table-hnsw-les-ecrivains-passent-un-par-un` ;
  - `un-bati-d-index-prend-tous-les-fils` ;
  - `un-create-vector-index-sous-ecritures-retient-les-autres` ;
  - `un-create-vector-index-sous-quarante-ecrivains-dure-quatre-minutes` ;
  - `une-ecriture-se-lie-avant-de-prendre-le-verrou` (un refus nommé à relancer).
- **Lenteur et mémoire** :
  - l'insertion HNSW (rétrécissements, lot après lot) ;
  - la suppression sous HNSW rempli par SET ;
  - l'`IN` sur la clé (contourné côté rag3weaver) ;
  - le prédicat d'un saut ;
  - les lignes hors du graphe ;
  - la mémoire d'un index et celle des BLOB.
- **`une-lib-d-une-autre-abi-se-charge-et-tombe-sur-un-symbole`** : un hôte qui ne démarre plus après un
  rebâti de la lib. Le magasin et `check-lib` l'évitent en pratique.

## 3. Pas atteignables par Lucie aujourd'hui : cas du banc, à garder tels quels

**À établir, sous vingt écrivains et un pool saturé** : le banc les produit, et un hôte de Lucie n'a pas cette
charge.
- `sous-vingt-ecrivains-le-pool-sature-et-ne-se-vide-plus` ;
- `sous-vingt-ecrivains-un-lecteur-meurt-dans-l-index-de-cle-primaire-sur-disque` ;
- `verifier-la-quarantaine-d-une-annulation-partielle` (aucun défaut reproduit).

**Le banc et le bâti seulement** : `un-temoin-de-temps-du-banc-rougit-sous-charge`,
`la-cible-clean-efface-la-grammaire-cypher`, la CI de master.

## 4. À dégrader en « confort », ou à fermer comme hors d'atteinte

| ticket | pourquoi | proposition |
|---|---|---|
| `2026-10-10-un-groupe-de-copies-bien-plus-grand-que-le-degre` | il faut des centaines de copies d'un même vecteur, et le plus grand groupe réel en compte 49 | garder en confort (déjà dit tel) |
| `2026-10-11-l-elagage-peut-couper-le-dernier-pont-vers-une-region` | vu sur une variante d'essai, jamais sur master en 13 essais | dégrader en « confort, non reproduit » |
| `2026-10-04-accent-grave-double-dans-un-nom` | aucun nom de rag3weaver n'a d'accent grave | garder en confort (déjà dit tel) |
| `2026-10-05-rtree-recoit-des-decalages-provisoires`, `2026-10-18-la-retention-des-index…` (part geo) | l'extension geo n'a aucun utilisateur, et l'index ne se crée pas | dégrader en confort, réveiller avec geo |
| `2026-10-04-recettes-de-l-amont-non-reproduites` | non reproduit depuis le 4 octobre | **fermer** comme non reproduit, rouvrir si un cas réel paraît |
| les estimations (`copy-de-relations-annule-gonfle…`, `analyze-au-seuil`, `analyze-non-transactionnel`, `hyperloglog-a-64-registres`, `copy-garde-un-doublon-selon-les-fils`) | seuls le planificateur et `STATS_INFO` les lisent ; aucune réponse fausse | garder en confort (déjà dit tels) |
| les optimisations du journal et du point de reprise (`un-flottant-coute-douze-octets`, `cout-par-cellule`, `point-de-reprise-reecrit-toute-une-table-de-blobs`, `journal-…-sans-borne-hors-copy`) | optimisations hors stèle | garder |

**En une ligne pour Lucie.** Sur tes bases, aujourd'hui, ce qui peut vraiment mal tourner :
1. **les orphelins du plein texte**, qui reviennent : correctif chez recherche ;
2. **le SIGSEGV d'optimisticRead**, déjà vu une fois en service ;
3. **une coupure au mauvais instant**, la durabilité étant en cours ;
4. **une extension qui ne se charge pas** et efface un index sans le dire.

Le reste est corrigé dans la lib posée, réparé cette nuit, ou propre à la charge du banc.
