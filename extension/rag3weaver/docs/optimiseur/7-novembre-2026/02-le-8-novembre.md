# Le 8 novembre : le repli du portable, l'écart des deux sens, la pose de cbc06c3e1

Session G (optimiseur). Heures lues sur le poste où l'action a eu lieu. Toute heure ou mesure prise à
luciepc a sa lecture de charge à côté (`~/.cache/rag3weaver-build/lecture-poste.sh`) : Lucie joue
parfois depuis seat1, qui rend sur **card0 (07:00.0, GPU[1] pour rocm-smi)** ; les services
d'embarquement sont sur card2 (04:00.0, GPU[0]).

## Le matin : deux SIGSEGV sous dadbf0041, le repli

- L'hôte du portable est tombé deux fois sous dadbf0041 (8 h 50, 9 h 05) : `BufferManager::optimisticRead`
  par HNSW `shrinkForNode`, un use-after-free corrigé ensuite par cbc06c3e1.
- 9 h 06 : repli sur le binaire eb154bf5f et la lib 17a094d12. La base de code a refusé le rejeu de son
  journal : le rejeu ne sait pas mettre à jour une relation qu'un COPY a insérée plus tôt dans la même
  transaction. Le journal est cohérent, c'est le rejeu qui est en défaut (correctif A, 6a71f641d /
  426153052). L'original est gardé dans `~/.local/share/rag3weaver/preuves/2026-11-08-code-rag3db-depot-wal-dadbf0041/`.
  L'instantané avant-43 a été remis à 9 h 10 et la montre a rattrapé.

## L'écart des deux sens après un index complet

Sous cbc06c3e1 à luciepc, après un `index confirm` complet, le contrôle des deux sens a trouvé un écart :
INHERITS_FROM, QUERIES et LOCKS dans code-lourd ; QUERIES, IMPLEMENTS et LOCKS dans code-principal.
Les copies prises par hotfixs à 12 h 06 étaient à 0 écart.

Le départage (ordre de c7), à luciepc sous `poste lourd`, un run après l'autre, avec
`~/.cache/rag3weaver-build/departage.sh <étiquette> <binaire> <lib>`. Chaque run :
1. copie reflink neuve de `coupure-12h06/code-lourd` ;
2. réécrit le `.extensions` vers le libvector de la lib du run (la copie de 12 h 06 nomme celui de dadbf0041) ;
3. copie l'arbre des gabarits entier, parce que le manifeste a des chemins relatifs, et retire montre et bilans ;
4. hôte d'essai 7991 sous `LD_LIBRARY_PATH` et `RAG3DB_LIBRARY_DIR`, lib vérifiée dans les maps ;
5. `index confirm`, arrêt TERM, puis `controle-des-deux-sens.sh`.

Premier essai à 14 h 03 sans valeur : le manifeste copié seul perdait ses chemins relatifs, l'ouverture
était refusée et le « 0 écart » portait sur la copie intacte. Depuis, le script s'arrête en ROUGE si
l'ouverture ou l'index échoue, et ne contrôle qu'après « indexation terminée ».

| run | lib | binaire (md5) | index | verdict |
|---|---|---|---|---|
| 1b | 17a094d12 | avant-dadbf004 (41411ff0) | 14 h 26 – 15 h 07 | **ÉCART** : INHERITS_FROM 650/658, LOCKS 544/546, QUERIES 20093 des deux côtés mais 5 relations d'un seul côté, 6 vues deux fois, 9 aux bouts différents ; IMPLEMENTS égales |
| 2b | dadbf0041 | avant-cbc06c3e (fe994691) | 15 h 37 – 15 h 41 | **ÉCART**, mêmes chiffres que 1b |
| 3c | 04063ee06 | avant-43 (25de5a8d) | — | désarmé (contrordre de c7, 15 h 48) ; prêt pour hotfixs sur une copie de `instantanes/avant-43/code-lourd` (format 42) |

Le défaut existe donc sous 17a094d12 : c'est la famille LOCKS du 25 octobre, et hotfixs l'a retrouvée
sous 58b46a9bf. Il mène le chantier (ticket 2026-11-08-un-index-complet-croise-les-deux-sens-des-petites-tables-de-relations).
L'index du 1b a pris 41 min et celui du 2b 5 min : c'est la file de 7878 et non le moteur. Pendant un
run, `/embed` fait la queue pour tout le poste (e2e_lectures de hooks-essai rouges, « pas indexé en 60 s »
chez everything declarative). On annonce donc la fin de chaque phase d'index.

Le portable, contrôlé à 14 h 15 (instantané controle-portable-8nov, sous 17a094d12) : **aucun écart**.
Cette base n'avait pas eu d'index complet depuis 9 h 10 : la montre seule ne fait pas naître l'écart.

**Règle jusqu'au correctif moteur : aucun `index confirm` complet sur les bases de Lucie.**

## La pose de cbc06c3e1 au portable (15 h 41 – 15 h 44)

- Coupure annoncée pour 15 h 45, faite à **15 h 41** : j'avais daté un message sans lire l'heure.
- Binaire ff6e88ddf (d982b513, check-lib ece264d9 « même bâti »), l'ancien gardé en `.avant-cbc06c3e`.
- Instantané avant-cbc06c3e1 des deux bases ; copie de memoire-longue par hotfixs (« copié » à 15 h 42 min 11 s).
- `courant` = cbc06c3e1 aux deux postes (planchers tenus, md5 389935e1 des deux côtés, ide-fusion inactive).
- Réveil VERT à 15 h 42 min 30 s : MainPID 2098825, cbc06c3e1 dans les maps et dans l'env,
  LISTEN_FDS=1 ; tunnels 7979, 7980 et 7981 vivants.
- Bases rouvertes. Témoin de l'IDE : /ide/ rendu, 313 éléments, aucune erreur de console.
- `sandbox.env` remis dans l'arbre de c7 (commit 3d8a875 de code-essais-c7), backend rouvert.
- Disques : portable 340 Gio, luciepc 571 Gio.

## La repose de 16 h 08 (e59e59834)

Lucie essayait l'atelier depuis le portable et le pas 3 échouait : `open_branch` (9fd49b38d) est arrivé sur
master après la coupe de ff6e88ddf. J'ai bâti à luciepc la tête e59e59834 contre cbc06c3e1, en -j8 et sous
`poste lourd`, entre 16 h 04 et 16 h 05 (binaire 708a7861, check-lib cc338cd3). Hôte d'essai VERT, puis
check-manifests à 3 sur 3. Coupure de 16 h 08 min 16 s à 16 h 08 min 23 s, annoncée une minute avant.
Réveil VERT (MainPID 2243814), trois bases rouvertes, /ide/ rendu avec 363 éléments. L'ancien binaire
est gardé en `.avant-e59e5983`. Pas d'instantané neuf : même lib, mêmes bases.

7878 est libre depuis 16 h 08 : le départage est fini.

## À suivre

- La prochaine pose bâtit 36aced578 ou plus récent (lot d'archi : un nom cité entre accents graves dans
  le chat devient un lien). Le témoin y clique.
- Les bases code-lourd et code-principal de luciepc reviennent aux copies de 12 h 06 d'hotfixs, sans
  réindexation complète.
