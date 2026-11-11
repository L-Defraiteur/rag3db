# Rapport de session — hotfixs (seconde session du cœur C++), 11 novembre 2026

Écrit le 11 novembre 2026 à 16 h 30 par la session « hotfixs ». C'est la seconde session Claude du
cœur C++ de rag3db ; son nom pour les messages entre sessions était « coeur c++/hotfixs/tickets ».
Le texte s'adresse à un agent qui n'a rien suivi de nos échanges. Le vocabulaire propre au projet
est défini dans `02-knowledge-dump.md`, à côté de ce fichier. Les mots les plus courants :

- **rag3db** : le moteur de base de données graphe, en C++, dans `src/` (un fork de Kuzu) ;
- **rag3weaver** : le produit en Rust qui l'utilise, dans `extension/rag3weaver/` ;
- **le portable** : le poste de Lucie où tourne cette session (`~/git_workspaces/…`) ;
- **luciepc** : l'autre poste, joint par `ssh lucied@luciepc`, en shell fish ;
- **c7** : la session d'orchestration, qui fixe l'ordre du travail de toutes les sessions ;
- **le cœur** : la première session du cœur C++ ;
- **G** : la session « optimiseur », qui bâtit et pose les libs.

Je me suis arrêtée le 9 novembre vers 2 h 40 et j'ai repris le 11 novembre à 16 h. Entre les deux,
rien n'a tourné de mon côté : à luciepc, aucun fichier de `~/.cache/rag3db-tickets-notes/` n'est
plus récent que le 9 novembre à 3 h.

## 1. Le chantier principal : les deux sens d'une table de relations croisés après un index complet

**Le défaut.** Le 8 novembre, un `index confirm` complet (la réindexation entière d'un dépôt de code
par rag3weaver) laissait des relations fausses dans trois petites tables, INHERITS_FROM, QUERIES et
LOCKS. Une table de relations est rangée deux fois sur disque : le **sens avant**, une liste par
nœud source, et le **sens arrière**, une liste par nœud destination. Lus nœud par nœud, les deux
sens ne disaient plus la même chose : une relation rattachée au mauvais nœud, une présente d'un seul
côté, une vue deux fois, ou une rangée vide lue comme « relation 0 vers le nœud 0 ». Le défaut était
déterministe sous trois libs successives.

Le ticket : `docs/coeur-cpp/tickets/2026-11-08-un-index-complet-croise-les-deux-sens-des-petites-tables-de-relations.md`
(déplacé par le cœur le 11 novembre).

**Ce que j'ai fait, dans l'ordre :**

1. **Une hypothèse réfutée.** On soupçonnait un débordement de région dans
   `populateRegionCSROffsets` (`src/storage/table/csr_chunked_node_group.cpp`). Une lib d'étude
   copiait les deux `KU_ASSERT` sur stderr. Pendant tout un index, aucune garde n'a sauté, et les
   écarts étaient les mêmes. Branche `etude-debordement-csr-2`, à ne pas atterrir.
2. **La trace exacte.** Le binaire d'étude `trace-exacte-9bae` (2b73c4ff7, à ne pas atterrir) écrit
   chaque instruction Cypher avec ses paramètres remplacés par des littéraux, et garde une copie du
   CSV de chaque COPY. J'ai lancé un index complet de la copie saine sous la lib 9bae60786, sur un
   hôte d'essai (port 7992) avec mon service d'embarquement (port 7882). Résultat : 3 178
   instructions, 16 COPY, aucun BEGIN, et les mêmes écarts que tous les runs précédents, compte pour
   compte.
3. **Le rejeu.** Le programme `rejoue` (source dans `outils/rejoue.cpp`, ici) rejoue la trace sur
   une copie neuve, avec 0 erreur d'exécution. Le contrôle des deux sens est vert jusqu'à
   l'instruction 777 et rouge après la 797.
4. **C'est le point de reprise** (le CHECKPOINT, qui écrit sur disque l'état gardé en mémoire et
   dans le journal). « Instructions 1 à 796, puis CHECKPOINT » rougit, alors que l'état lu juste
   avant est vert.
5. **La dichotomie sur K** (le plus petit nombre d'instructions qui, suivies d'un CHECKPOINT,
   rougissent) : **K = 730**. « 1 à 729 puis CHECKPOINT » est vert, « 1 à 730 puis CHECKPOINT »
   est rouge. La dichotomie a tourné pendant le fstrim de luciepc : on ne lit que ses verdicts, pas
   ses temps.
6. **Le témoin réduit.** Sur la base « 1 à 729 puis CHECKPOINT », une seule relation de
   l'instruction 730 suffit. Elle existait déjà : le MERGE la retrouve et réécrit sa propriété
   `line`, puis CHECKPOINT, et le contrôle rougit. J'ai rebâti le cas de zéro (`outils/synth.cpp`),
   puis réduit les 659 paires par ddmin (réduction automatique d'un cas rouge, en 33 essais) :
   **trois relations suffisent**. Dans une table Scope de 82 890 nœuds, on verse par COPY
   37654→23480, 5358→5354 et 6316→640 ; puis CHECKPOINT ; MERGE…SET sur 37654→23480 ; CHECKPOINT.
   Les deux sens sont alors croisés, avec la signature de la base de Lucie.
7. **Le gtest.** `LateRelTableRewrites.UpdatingACopiedRelKeepsBothDirectionsThroughACheckpoint`,
   dans `test/transaction/concurrence/late_rel_table_rewrites_test.cpp`, de la cible
   `concurrence_test`. Je l'ai poussé sur la branche `temoin-deux-sens-reprise` (dfc0ec8b3). Le
   cœur l'a repris et l'a joué rouge avant son correctif, vert après. Il est sur master en
   e3afa05eb. **Je ne l'ai jamais vu tourner moi-même** : mes lancements à luciepc passaient par
   `g1.sh`, qui ignore la variable `CIBLE`, et ont joué `transaction_test`, où le filtre ne trouve
   aucun test (le script le signale « ROUGE : aucun test joué »).

**La cause et le correctif** (cœur, sur master en **29a7bc52a** le 9 novembre à 10 h 30). Le point
de reprise d'une région CSR remontée à un niveau plus haut recalcule ses décalages : ses trous sont
regroupés en fin de région. Mais `checkpointColumn` (`src/storage/table/csr_node_group.cpp`) sautait
toute colonne sans mise à jour dans la région. Le voisin, l'identifiant de relation et les autres
propriétés restaient donc aux anciens décalages, et le nouvel en-tête les lisait ailleurs. Le
correctif ajoute `csrOffsetsMoveInRegion` : une colonne se réécrit aussi quand un décalage ou une
longueur change dans la région. ASAN (le détecteur d'erreurs mémoire), passé par le cœur sur ma
recette, n'a rien trouvé : c'est une erreur de logique, pas un accès mémoire fautif.

**Après le correctif** (d'après le rapport de c7) : G a bâti la lib 29a7bc52a. Le 9 novembre
entre 11 h 27 et 11 h 49, un index complet d'une copie de code-lourd, passé par le vrai chemin de
l'hôte et embarqué par mon service 7882, a donné **0 écart** (658/658, 546/546, 20 093/20 093). La
lib a été posée à luciepc à 12 h 12, puis sur le portable.

## 2. Le rejeu de la trace sous 29a7bc52a : pas fait

c7 m'avait demandé de rejouer ma trace exacte sous la lib du correctif. Le 9 novembre vers 11 h,
ma session attendait une approbation de Lucie (relevé par c7 à 10 h 56). Elle s'est arrêtée avant
de lancer quoi que ce soit. **Ce rejeu n'a pas tourné, et je n'en ai aucun verdict.** Le 0 écart
de 11 h 49 (ci-dessus) vient d'un index complet passé par l'hôte, pas de ma trace.

**Le ticket de l'index complet est toujours ouvert sur master.** Son État dit : « corrigé sur master
en 29a7bc52a (9 novembre, cœur c++), à fermer après la pose de la lib et un index complet vert sur
une copie ». Les deux conditions sont remplies d'après c7 (lib posée, 0 écart à 11 h 49). Le cœur
devait le fermer ; il ne l'a pas fait. Ma trace rejouée sous 29a7bc52a serait une seconde preuve,
pas une condition.

## 3. Les hotfixs en cours (lots arrêtés avant leur push)

Un **lot** est un ensemble de commits poussé d'un coup sur master après une passe verte. Une
**passe** est le jeu complet des tests concernés (voir le knowledge dump). Aucun de ces lots n'est
sur master :

| Branche | Dernier commit | Ce que c'est | Ce qui manque |
|---|---|---|---|
| `versement-verrou-2` | 4510f4631 | B : le verrou de versement. Un second écrivain qui verse dans une table attend le verrou de l'index au lieu de croiser l'écriture (`IndexLookup::acquireLocksBeforeExecution`, `acquirePipelineLocks` dans `processor.cpp`, témoin rouge 288f81d51). | La passe complète. Rejeu ciblé vert à 24/24 le 8 novembre ; j'ai arrêté la passe complète moi-même, sur la priorité donnée par c7 aux deux sens. |
| `g5-statistiques` | 13b8a6d57 (distant) ; le worktree local est à e43ebb920 | G5 : les statistiques de table, avec la mesure de l'ANALYZE au seuil (≈ +390 ms une fois sur code-lourd). | La passe complète, puis le push. |
| `agent/tests-bases-dans-tmp` | fe16dad5b | Le troisième lot de l'équipier Gemini : chaque base de test vit dans un dossier temporaire à elle, effacé à la fin (`BaseTemporaire`). | La passe sous la lib du moment. À coordonner avec le témoin des résidus de recherche : sortir de `suites_qui_laissent_des_residus.txt` les suites que le lot rend propres. |
| `reindex-complet-temoin` | da1f9b1b6 | Cinq témoins verts de formes d'index (`late_rel_table_rewrites_test.cpp`). Aucun n'a reproduit le défaut. | Décider s'ils atterrissent. Ils protègent des chemins voisins, mais aucun n'était rouge. |
| `ci-cassert-geo-2` | c6c84af6a | Déjà sur master (le format du lot A, et le ticket de la CI). | Rien ; le worktree peut partir. |

**La fenêtre de formatage clang-format-18** (passer `clang-format-18` sur tout le code C++ en un
commit de format pur, pour remettre la CI au vert) devait avoir lieu dans la nuit du 8 au 9
novembre. Elle n'a pas eu lieu de mon côté, et master n'en porte aucune trace. Le ticket :
`docs/coeur-cpp-hotfixs-tickets/tickets/2026-10-25-la-ci-de-master-est-rouge-depuis-le-18-octobre-cassert-dans-geo-et-format-clang.md`,
point 2.

## 4. Ce qui reste sur disque

Je n'ai rien effacé. Tout ce qui suit est à moi et peut partir quand la suite est décidée.

**À luciepc**

- **Service d'embarquement 7882** : `rag3weaver-embeddings --adresse 127.0.0.1:7882`, pid 912225,
  lancé le 8 novembre depuis `~/.cache/rag3db-mesure-g5/embed-7882` ; il tourne toujours. On
  l'arrête par son pid, vérifié avant : `ps -o pid,args -p 912225` doit montrer cette ligne, puis
  `kill -TERM 912225`.
- **Aucune unité systemd à moi** (vérifié le 11 novembre). Aucun hôte d'essai ne tourne sur 7992.
- **Les notes et scripts** : `~/.cache/rag3db-tickets-notes/` (992 Mio). On y trouve :
  - la trace `trace-exacte-9bae/cypher.txt` (345 Mo), avec `copies/` (les CSV des COPY) ;
  - les traces réduites `d-796-checkpoint.txt`, `f-729.txt`, `g-730.txt`, `item-0.txt` à
    `item-6.txt`, et le journal de la dichotomie `bissecte.log` ;
  - les scripts `departage-trace.sh`, `trace-exacte-9bae.sh`, `rejeu-9bae.sh`, `bissecte.sh`,
    `ddmin.py`, `bati-trace-9bae.sh`, `g1.sh`, `g1c.sh`, `equipier-lot.sh` ;
  - les sources `rejoue.cpp`, `synth.cpp`, `croise.cpp`, `requete.cpp`, avec leurs binaires
    `synth` et `requete-9bae` ; `paires-729.txt` et `paires-min.txt`.
- **Les binaires d'étude** : `~/.cache/rag3db-mesure-g5/trace-9bae/` (`rag3weaver-backend-trace` et
  `rejoue`, liés à 9bae60786) et `~/.cache/rag3db-mesure-g5/lib-etude/` (la lib d'étude à gardes).
  Ils ne sont pas dans `~/.local/bin`.
- **Les copies de bases**, toutes des copies, aucune base vivante :
  - `~/.cache/rag3db-locks/coupure-12h06/` (2,2 Gio) : la copie saine de code-lourd et
    code-principal, prise le 8 novembre à 12 h 06, 0 écart. C'est la source de tous les rejeux.
  - `~/.cache/rag3db-locks/fixture-729/` (1,3 Gio) : « 1 à 729 puis CHECKPOINT », verte.
  - `~/.cache/rag3db-locks/fixture-796/` (1,3 Gio) : déjà rouge, sans usage.
  - `~/.cache/rag3db-locks/rejeu/` (22 Gio) : les copies des rejeux et de la dichotomie.
  - `~/.cache/rag3db-locks/sonde-croise/` (4,5 Gio) et `sonde-fixture/` (1,3 Gio) : les copies des
    sondes.
  - `~/.cache/rag3db-locks/synth/` (35 Mio) et `ddmin/` (vide).
  - `~/.local/share/rag3weaver/instantanes/hotfixs-trace-{etude,exacte,9bae}` et leurs `-controle`
    (2 Gio chacun) : les bases des index tracés et leurs contrôles. `hotfixs-trace-exacte` est le run
    arrêté à mi-index le 8 novembre, sans valeur.
- **Les targets cargo** : `~/.cache/rag3weaver-build/target-hotfixs-etude` (1,4 Gio) et
  `target-equipier` (12 Gio).
- **Les worktrees** : `~/git_workspaces/rag3db-dbg` (détaché à dfc0ec8b3, avec trois fichiers non
  suivis `follows.csv`, `user.csv`, `user.parquet` laissés par des tests ; son `build/moteur` sert à
  `g1.sh`) et `~/git_workspaces/rag3db-equipier`.

**Au portable**

- **Les worktrees de mes lots**, tous propres (rien de modifié), dans `~/git_workspaces/` :
  `rag3db-verrou` (versement-verrou-2), `rag3db-g5` (g5-statistiques, en retard sur sa branche
  distante), `rag3db-tmpb` (agent/tests-bases-dans-tmp), `rag3db-reindex` (temoin-deux-sens-reprise),
  `rag3db-etude` (trace-exacte-9bae), `rag3db-g3`, `rag3db-rejeu`, `rag3db-cassert`, `rag3db-g2c` (déjà
  atterris ou abandonnés), `rag3db-docs-cc` (mes docs) et `rag3db-tickets-script` (le script des
  tickets, atterri en f154356ea).
- **Les copies de travail** : `~/.cache/rag3db-locks/trace-etude/` et `memoire-13h40/`.

## 5. Ce qui est bloqué, et sur quoi

- **Le rejeu de ma trace sous 29a7bc52a** attendait une approbation de Lucie le 9 novembre.
  Personne ne l'a relancé depuis.
- **B, G5 et le lot de l'équipier** attendent chacun une passe complète. La règle : un push ne part
  que derrière une passe verte, sans rouge dit « d'environnement ».
- **La fenêtre de formatage** attend une heure calme et le mot de c7.

## 6. Les trois prochains pas

1. **Fermer le ticket de l'index complet**, ou demander au cœur de le fermer. Ses conditions sont
   remplies d'après c7 (lib 29a7bc52a posée, 0 écart par l'hôte). Si l'on veut une seconde preuve,
   rejouer ma trace sous 29a7bc52a (section « Reprendre » A).
2. **Mener B (`versement-verrou-2`) jusqu'au push** : rebase sur master, passe complète, push dans un
   `if` qui ne pousse que sur VERT (section « Reprendre » B).
3. **Puis G5 et le lot de l'équipier**, de la même façon. Ensuite la fenêtre de formatage, au mot de
   c7.

## 7. Reprendre

Chaque travail inachevé, avec son chemin exact et les commandes pour repartir de là, pas de master.
Rappels :
- toute commande à luciepc passe par `ssh -T lucied@luciepc 'bash -s' <<'EOF' … EOF`, car luciepc
  est en fish ;
- un travail lourd (bâti, passe, index) passe sous `~/.cache/rag3weaver-build/poste lourd …`, le
  verrou des postes ;
- la lib courante est 29a7bc52a, le dossier
  `~/.cache/rag3db-libs/rag3db-build-29a7bc52aaf11e6204f9beafaffb154105417e13-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF`
  aux deux postes (le lien `courant` y pointe).

### A. Le rejeu de la trace exacte sous 29a7bc52a (luciepc)

- **Fait** : la trace (`~/.cache/rag3db-tickets-notes/trace-exacte-9bae/cypher.txt`) et le
  rejoueur, mais tous deux liés à 9bae60786, la lib d'avant le correctif.
- **Reste** :
  - poser la trace sur 29a7bc52a : la branche `trace-exacte-9bae`, worktree `~/git_workspaces/rag3db-etude`
    au portable, a un seul commit (2b73c4ff7) au-dessus de 9bae60786 ;
  - rebâtir le binaire et le rejoueur contre 29a7bc52a ;
  - rejouer `d-796-checkpoint.txt` sur une copie de `coupure-12h06/code-lourd` et `item-2.txt` sur
    une copie de `fixture-729`, les deux attendus verts ;
  - éventuellement un index tracé complet, attendu à 0 écart.
- **Commandes** :

```bash
# au portable : la trace posée sur la lib du correctif, sous un nom neuf (jamais de push en force)
cd ~/git_workspaces/rag3db-etude && git fetch -q origin && git checkout -q --detach 29a7bc52a \
  && git cherry-pick 2b73c4ff7 && git push -q origin HEAD:refs/heads/trace-exacte-29a7
# à luciepc : copier bati-trace-9bae.sh en bati-trace-29a7.sh, y remplacer ID par le dossier 29a7bc52a,
# la branche trace-exacte-9bae par trace-exacte-29a7 et D par ~/.cache/rag3db-mesure-g5/trace-29a7 ;
# même chose pour rejeu-9bae.sh (L et X). Puis :
ssh -T lucied@luciepc 'bash -s' <<'EOF'
N=~/.cache/rag3db-tickets-notes; T=$N/trace-exacte-9bae
TRACE=$T/d-796-checkpoint.txt bash $N/rejeu-29a7.sh r29-d796 0 0
SRC=~/.cache/rag3db-locks/fixture-729 TRACE=$T/item-2.txt bash $N/rejeu-29a7.sh r29-item2 0 0
EOF
# attendu : « aucun écart » et « code 0 » sur les deux
```

### B. Le verrou de versement (portable, `~/git_workspaces/rag3db-verrou`)

- **Branche** `versement-verrou-2`, dernier commit 4510f4631, worktree propre.
- **Fait** : le témoin rouge, le correctif, la prémisse du test de G1 adaptée, le ticket fermé, les
  nouveautés. Rejeu ciblé vert à 24/24 le 8 novembre.
- **Reste** : rebaser sur master (qui a reçu 29a7bc52a et d'autres lots depuis), rebâtir, passe
  complète du moteur (cibles de test C++ de `transaction`, `concurrence` et du banc), passe Rust de
  rag3weaver sous la lib bâtie, puis pousser par `outils/push-lot.sh`.
- **Commandes** :

```bash
cd ~/git_workspaces/rag3db-verrou && git fetch -q origin && git rebase -q origin/master
# un rebase arrêté sur un conflit se lit avant tout push : git status
git push -q origin HEAD:refs/heads/versement-verrou-3      # nom neuf après un rebase
# puis la passe à luciepc : les scripts b-passe.sh et g1-lot.sh de ~/.cache/rag3db-tickets-notes
# montrent la forme (ils lisent une branche, bâtissent dans ~/git_workspaces/rag3db-dbg sous poste lourd,
# écrivent VERT ou ROUGE en dernière ligne). Push seulement dans un `if` qui exige VERT :
bash outils/push-lot.sh ~/git_workspaces/rag3db-verrou <hash de master rejoué par la passe>
```

### C. G5, les statistiques (portable, `~/git_workspaces/rag3db-g5`)

- **Branche** `g5-statistiques`. Le distant est à 13b8a6d57, le worktree local à e43ebb920 :
  commencer par `git pull --ff-only`.
- **Reste** : rebase sur master, passe complète, push. Même forme que B.

### D. Le troisième lot de l'équipier (portable, `~/git_workspaces/rag3db-tmpb` ; luciepc, `~/git_workspaces/rag3db-equipier`)

- **Branche** `agent/tests-bases-dans-tmp`, dernier commit fe16dad5b ; target cargo
  `~/.cache/rag3weaver-build/target-equipier` à luciepc.
- **Reste** :
  - rebase sur master ;
  - passe par `equipier-lot.sh` (luciepc), avec `LIB` réglée sur 29a7bc52a et sans
    `MOTEUR_ANCIEN`, puisque le moteur a bougé depuis ;
  - sortir de `suites_qui_laissent_des_residus.txt` les suites que le lot rend propres ;
  - push.

### E. Le script des tickets : atterri

`extension/rag3weaver/scripts/check_tickets.py` lit maintenant `docs/<session>/tickets/` (f154356ea,
11 novembre). Rien à reprendre. Le worktree `~/git_workspaces/rag3db-tickets-script` peut partir.
