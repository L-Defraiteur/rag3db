# Rapport de session — cœur C++, nuit du 1ᵉʳ au 2 octobre 2026

Session « cœur C++ » : moteur, journal, point de reprise, lecteurs et
écrivains concurrents. Travail commandé par l'orchestrateur (« rag3weaver
archi ») au nom de Lucie. Le savoir de fond est dans `02-knowledge-dump.md`,
à côté.

## En cinq lignes

1. **Un défaut de reprise après panne, hérité de Kuzu, est corrigé et
   prouvé** : une base dont une table avait déjà connu des points de reprise
   ne se rouvrait plus si un point de reprise était interrompu. Commit
   `df4838542`, accepté, en cours de livraison sur master par la session
   « produit ».
2. **La course du lecteur d'un autre processus est comprise et rendue
   déterministe** ; sa correction (la marche 1) est écrite, verte sur ses
   tests ciblés, **pas encore livrée**.
3. **Trouvaille la plus lourde : après un point de reprise échoué, le
   processus continue et lit faux**, en silence, puis plante. C'est l'état de
   master, pas un effet de mon correctif. rag3weaver y est exposé. Rien n'est
   corrigé là-dessus.
4. Le plan des écritures parallèles (cibles A et B) est écrit et poussé :
   `docs/2-octobre-2026-00h17/01-ecritures-paralleles-vela-et-le-chemin.md`.
5. Restent dus, dans cet ordre : finir la marche 1, relire le banc de
   concurrence, le refus après point de reprise échoué, la note sur les
   verrous, A2 à A4, la section HNSW.

---

## 1. Ce qui a été fait

### 1.1 L'étude de Vela et le plan des écritures parallèles

Rapport de 663 lignes sur master (`35654d401`) : où en est Vela, ce que
« concurrent writes » veut dire dans leur code, ce qu'on reprend à la main
plutôt que de fusionner, la cible A (plusieurs écrivains dans le processus,
marches A1 à A8, environ trois semaines), la cible B (plusieurs processus
écrivains, journal partagé), la découpe en plusieurs fichiers par genre
d'index, les garde-fous. Rien de plus à en dire ici : c'est le document à lire.

### 1.2 La cause de la course du lecteur

Une ouverture en lecture seule ne prend aucun verrou et rejoue en mémoire le
journal de l'écrivain vivant, par trois lectures à trois instants différents.
Un point de reprise qui passe entre deux d'entre elles donne une erreur
variable — ou **aucune erreur et un contenu faux** : un lecteur a vu 18
relations là où il y en a 9.

Pour le prouver sans dépendre du hasard, j'ai posé un crochet de test dans
`WALReplayer` : l'écrivain, dans le même processus, agit exactement entre deux
lectures du lecteur. Commit `b254776d2` (branche
`lecteur-reverifie-a-l-ouverture`) : le crochet et les tests rouges.

**Rectification** : ma conclusion du 18 septembre (les refus du lecteur
seraient une famine d'ordonnancement) n'est plus soutenue. Le test de ce
jour-là comptait les refus sans lire leur message.

### 1.3 Le défaut du tableau sur disque, et son correctif

En cherchant pourquoi la revérification ne rendait pas le test de charge
entièrement vert, j'ai trouvé un défaut plus ancien et plus grave, **sans
rapport avec les lecteurs** : pendant un point de reprise, l'index de clé
primaire réécrivait **en place** des pages qui existaient déjà, alors
qu'elles doivent passer par le fichier fantôme. Une panne à ce moment laisse
un fichier de données à moitié avancé, et la base ne se rouvre plus (« Found
duplicated primary key »).

- Cause, critère retenu, mesure : `02-knowledge-dump.md`, §6, et le message du
  commit.
- Commit `df4838542`, branche `reprise-apres-panne-index-cle-primaire`.
- Suites, Release, sur ce commit : `transaction_test` 56/56, `copy_tests`
  19/19, stockage 77/77, `api_test` 101/102 — le seul rouge est
  `LecteursConcurrents.CeQueLeLecteurVoitEstCoherent`, le rouge connu de
  master, objet de la marche 1.
- Le fichier était identique à l'amont : **Kuzu, Vela et Ladybug portent ce
  défaut.**

### 1.4 La panne *pendant* la phase de stockage

Demandée par l'orchestrateur pour lever un « argument, pas preuve ». Commit
`a486fde9c`, sur la même branche : le douzième point de reprise est refusé à
sa k-ième allocation, pour chaque k, jusqu'à ce qu'il réussisse ; après chaque
panne on rouvre et on vérifie le contenu exact de deux tables.

- 180 points de reprise interrompus, dont 174 pendant la phase de stockage.
- Vert avec le correctif ; **rouge sans** (« Found duplicated primary key
  value 278 » à la réouverture).
- `FlakyBufferManager` est sorti dans
  `test/include/test_helper/flaky_buffer_manager.h` ; `copy_tests` 19/19 après
  le déplacement.
- Coût : 38 secondes (`transaction_test` passe de 27 à 65 s).
- Limite honnête : sur la version sans correctif, le test s'arrête par une
  exception à la réouverture ; je n'ai donc pas le numéro de l'allocation
  fautive. Qu'elle soit dans la phase de stockage est **déduit** du temps
  écoulé (onze secondes sur trente-huit), pas lu.

### 1.5 Après un point de reprise échoué, le processus lit faux

Trouvé en vérifiant que mon correctif ne cassait rien dans le même processus.
Expérience : mêmes douze cycles, panne après la phase de stockage, **sans
rouvrir**.

| | attendu | obtenu |
|---|---|---|
| `COUNT(a)` | 300 | 300 |
| `COUNT(DISTINCT a.id)` | 300 | **275** |
| `SUM(a.id)` | 44 850 | **37 675** |
| requête suivante | — | **SIGSEGV** dans `StringColumn::scanUnfiltered` |

**Résultat identique avec et sans mon correctif** : c'est l'état de master.
Cause lue : seules les transactions de point de reprise lisent le fichier
fantôme ; ce qu'un point de reprise raté y a mis devient invisible, alors que
les structures en mémoire ont avancé. Je n'ai pas vérifié si l'amont le porte.

La reprise par **réouverture**, elle, est juste : c'est ce que prouvent les
tests de 1.3 et 1.4.

Le test d'expérience est rangé ici :
`experience-meme-processus-apres-point-de-reprise-echoue.patch` (s'applique
sur `test/transaction/checkpoint_test.cpp` de `df4838542`).

### 1.6 La marche 1 : le lecteur revérifie à l'ouverture

Le lecteur retient la page 0 et l'empreinte du journal avant de lire,
revérifie après, et refuse par une erreur nommée si un point de reprise est
passé :

> `The database was checkpointed by another process while this read-only open
> was reading it […] Retry the open.`

Écrit, commité **« en cours »** (`6bc8ce632`), poussé. Tests ciblés verts sur
cet arbre : `ReadOnlyOpenTest.*` 6/6, `FlakyCheckpointerTest.*` 11/11. **Les
suites complètes n'ont pas été rejouées** depuis le rebasage sur le correctif.

---

## 2. Où exactement je me suis arrêtée

| | |
|---|---|
| Arbre | `/home/lucied/git_workspaces/rag3db-moteur`, propre (rien de non commité hors fichiers non suivis) |
| Branche extraite | `lecteur-reverifie-a-l-ouverture` @ `6bc8ce632`, poussée |
| Autre branche | `reprise-apres-panne-index-cle-primaire` @ `a486fde9c`, poussée |
| Dossier de build | `build/moteur` — **binaires mélangés** : `transaction_test` et `copy_tests` sont ceux de `a486fde9c`, `api_test` est celui de la marche 1. Tout reconstruire avant de se fier à un binaire |
| Aucun build en cours | — |
| Rien dans master de ma part cette nuit | hors ces deux documents et le rapport de fond |
| Reste dans `git stash` | une entrée « marche1-etapeB+correctif » : **obsolète**, tout est commité ; à jeter à la reprise |
| Worktree des documents | `/home/lucied/git_workspaces/rag3db-docs-arret` (détaché) : à retirer par `git worktree remove` |

Mon scratchpad est dans `/tmp` et disparaîtra : rien d'utile n'y reste qui ne
soit commité ou rangé ici.

**Pour reprendre la marche 1 :**

```sh
cd /home/lucied/git_workspaces/rag3db-moteur
git rebase reprise-apres-panne-index-cle-primaire   # a486fde9c ; conflit possible dans checkpoint_test.cpp
ninja -C build/moteur -j8 transaction_test api_test copy_tests \
  buffer_manager_test column_chunk_metadata_test compression_test \
  local_hash_index_test node_insertion_deletion_test node_update_test \
  rel_tests string_finalize_test
./build/moteur/test/api/api_test > api.log 2>&1                 # 102 attendus verts
./build/moteur/test/transaction/transaction_test > trx.log 2>&1
# le test de charge, dix passes :
./build/moteur/test/api/api_test --gtest_filter='LecteursConcurrents.*' --gtest_repeat=10 > charge.log 2>&1
```

Puis, avant de livrer la marche 1 : mesurer le coût de la somme de contrôle
sur un gros journal ; mesurer si un lecteur peut ne jamais entrer sous des
points de reprise très fréquents (vu une fois : zéro ouverture réussie sur une
passe) ; dire si le test Rust `un_lecteur_qui_insiste_pendant_qu_on_ecrit`
(`tests/e2e_prise_atomique.rs:331`, `#[ignore]`) peut être réactivé.

---

## 3. L'ordre de la reprise

Fixé par l'orchestrateur ; je n'en ai commencé aucun de neuf.

1. ~~La variante « pendant la phase de stockage »~~ — **faite** (`a486fde9c`),
   à relire.
2. **La marche 1** : rebaser, rejouer les suites, mesurer, livrer la branche.
3. **Relire le banc de concurrence** (`banc-de-concurrence`, `37a44e351`) : C1
   à C3 testent-ils ce que A2, A3, A4 corrigeront ; la barrière tient-elle la
   fenêtre ; le vérificateur peut-il rendre un faux vert. Répondre à la
   session « banc », et une ligne à l'orchestrateur : fusionnable ou non.
   Y compris sa remarque sur
   `ConcurrentRelationshipUpdatesWithMixedTransactions`
   (`transaction_test.cpp:639`). **Pas commencé.**
4. **Le refus après un point de reprise échoué** : branche à part, test
   d'abord (le patch d'expérience en est le rouge), erreur nommée jusqu'à
   réouverture, et la consigne pour l'appelant. D'abord vérifier **dans la
   documentation de PostgreSQL** ce qu'il fait d'un échec d'écriture ou de
   fsync pendant un checkpoint — je l'ai dit de mémoire (arrêt puis reprise
   par le journal), **ce n'est pas vérifié**.
5. **La note de conception sur les verrous** (verrou par clé, le second
   attend ; prise annoncée en tête de transaction ; même ligne et colonnes
   différentes ; interblocages ; interface prête pour B), en partant du
   comportement documenté de PostgreSQL et de Neo4j. Puis A2, A3, A4 contre le
   banc — rien à coder sur A3/A4 avant que Lucie ait vu la note.
6. **La section HNSW sous plusieurs écrivains** : le rapport de l'agent est
   reçu (résumé au knowledge dump, §10), pas revérifié, pas écrit.

---

## 4. Ce qui attend une décision de Lucie

1. **Signaler le défaut à l'amont ?** Kuzu, Vela et Ladybug le portent. Le
   message du commit `df4838542` est prêt à être envoyé. C'est une prise de
   parole publique : je ne l'ai pas faite.
2. **Le test de balayage coûte 38 secondes.** Le garder entier, ou balayer
   avec un pas ? Mon avis : le garder entier tant que la suite reste sous deux
   minutes — c'est lui qui a interrompu 174 fois la phase de stockage.
3. **Où ranger ces documents.** La règle habituelle met ce qui touche au cœur
   C++ dans `docs/` à la racine ; l'orchestrateur a ouvert le dossier de
   l'arrêt sous `extension/rag3weaver/docs/` pour toutes les sessions, et je
   l'ai suivi. À déplacer si la règle prime.

Ce qui **n'attend pas** sa décision, par le principe « comme les moteurs
établis, sauf mieux » : le refus après un point de reprise échoué (point 4
ci-dessus), s'il se confirme que c'est le comportement de PostgreSQL.

---

## 5. Ce qui n'est pas vérifié

- Les suites complètes sur la marche 1 après rebasage ; le coût de la somme de
  contrôle ; la famine du lecteur.
- Le relevé des autres sites d'écriture (« jumeaux ») vient d'un agent, en
  lecture seule ; je n'en ai revérifié que trois lignes (knowledge dump, §5).
- Trois soupçons sur le retour arrière d'un point de reprise échoué
  (`overflow_file.cpp:261`, `checkpointInMemory` pendant la phase de stockage,
  fantôme non vidé) : non suivis de bout en bout. Si le point 4 de la reprise
  est fait — le processus ne continue plus —, ils deviennent sans objet ;
  sinon ils sont à inscrire au journal des chantiers.
- Que l'amont porte lui aussi le défaut du §1.5 : non regardé.
- Le banc de concurrence : non relu.
- Le rapport de l'agent sur HNSW : non revérifié.
