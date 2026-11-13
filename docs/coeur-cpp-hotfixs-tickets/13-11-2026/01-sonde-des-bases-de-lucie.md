# La sonde des bases de Lucie : relations orphelines, morceaux sans parent et mots perdus de lucivy (13 novembre 2026)

Session hotfixs, à la demande de c7 (l'orchestration) et du cœur (la première session du cœur C++).
Le texte suppose qu'on n'a rien suivi.

## Pourquoi

Le cœur a reproduit en C++ un défaut grave du moteur, en trois temps :
1. un paquet d'insertions est défait par ROLLBACK ;
2. la base est rouverte ;
3. un COMMIT ultérieur réussit.

Ce COMMIT peut perdre des nœuds validés, insérés aux places du paquet défait, en gardant leurs
relations. On perd ainsi des données en silence, et il reste des **relations orphelines**, dont un
bout ne désigne plus aucun nœud, ainsi que des **morceaux** (les nœuds `*_Chunk` qui découpent le
texte d'un Scope pour la recherche) dont le parent manque.

**La cause, trouvée ensuite par le cœur** (même soir) : ce n'était pas le paquet défait.
`LocalStorage::commit` parcourait la `std::unordered_map` de ses tables locales pendant que l'index
vectoriel (HNSW) y créait les siennes, en validant sa table. Quand la table de hachage se
réorganisait, des tables de nœuds étaient sautées : leurs nœuds n'étaient jamais publiés, le COMMIT
réussissait, et leurs relations restaient orphelines. Le témoin `CommitLocalTablesTest` rougit dès
douze tables dans une transaction. Le ticket est
`docs/coeur-cpp/tickets/2026-11-13-un-commit-sous-index-vectoriel-saute-des-tables-de-noeuds.md`, et
le correctif est sur la branche `insertion-defaite-reouverture`. La sonde compte les orphelines dans
l'état des bases, quelle que soit leur cause : ses zéros valent pour ce défaut-là.

La question posée : les deux bases de code de Lucie, **code-lourd** et **code-principal**, en
portent-elles ?

## La réponse

**Non : 0 relation orpheline et 0 morceau sans parent, dans les deux bases (et, à la seconde lecture,
dans memoire-longue et code-rag3db-depot), dans toutes les tables et
dans les deux sens.** La lecture vaut pour l'état des bases au 11 novembre 2026 à 22 h 33, la
dernière écriture de leurs fichiers (voir « Ce qui a été lu »). Les seules limites sont dites plus bas.

La sonde voit bien le défaut quand il est là. Sur une copie de la base de reproduction du cœur
(`~/.cache/rag3db-moteur-notes/insertion-defaite/base/rollback.rag3db`, au portable), elle compte :
- 128 morceaux ScopeContent_Chunk sans parent ;
- 128 relations ScopeContent_CHUNKED_FROM dont la destination n'existe plus ;
- 96 relations DEFINES et 96 relations MENTIONS dont la source n'existe plus.

Elle rend le code 1 quand elle trouve un orphelin. Sur `a-seul/base`, l'autre base du cœur, elle
compte 0 orphelin.

## Seconde lecture : l'instantané avant-835d5298f (13 novembre, 20 h 08)

G a pris l'instantané `avant-835d5298f` hôtes arrêtés : à luciepc à 20 h 06 (code-lourd,
code-principal) et au portable à 20 h 07 (memoire-longue, code-rag3db-depot). Les fichiers de base y
datent du 13 novembre à 16 h 22. Même sonde (binaire md5 f65c5a70b673), même lib 835d5298f, même
lecture seule, sur des copies.

| base | poste | orphelins | paires au sens non prouvé |
|---|---|---:|---|
| code-lourd | luciepc | **0** | Symbol_CHUNKED_FROM, avant (table d'ancrage vide) ; **comptes identiques**, table par table, à la première lecture |
| code-principal | luciepc | **0** | Symbol_CHUNKED_FROM, avant ; **comptes identiques** à la première lecture |
| code-rag3db-depot | portable | **0** | Symbol_CHUNKED_FROM, avant |
| memoire-longue | portable | **0** | quatre au sens avant, voir ci-dessous |

Pour memoire-longue :
- **ETAT_DE, LIT et Ref_CHUNKED_FROM** n'ont aucune donnée sur disque : `CALL storage_info(…)` ne
  rend rien, et leurs tables d'ancrage Etat et Ref_Chunk sont vides.
- **CITE** (Memory → Ref) range ses deux sens, mais le planificateur part toujours de Ref, quelle que
  soit la forme d'ancrage essayée (filtre, égalité, `WITH`). Au sens arrière, la sonde lit 1
  relation, intacte. Le sens avant range 2 rangées : une relation vers une Ref disparue y échapperait.
  C'est la seule part de cette base non couverte.

Copies et relevés : à luciepc dans `~/.cache/rag3db-locks/sonde-orphelins-835d/`, au portable dans
`~/.cache/rag3db-locks/sonde-orphelins/portable-835d/`.

## Ce qui a été lu (première lecture)

- **Les bases** : des **copies** (`cp -a --reflink=always`), avec fichier, extensions et `.fts`
  ensemble, et sans journal `.wal`. Elles viennent de l'instantané de G
  `~/.local/share/rag3weaver/instantanes/avant-207701bb2/` à luciepc, pris l'hôte arrêté le
  13 novembre à 15 h 51 (`fuser` vide avant la copie). Les copies sont dans
  `~/.cache/rag3db-locks/sonde-orphelins-13nov/{code-lourd,code-principal}/` à luciepc. **Rien n'a
  touché les bases de Lucie elles-mêmes.**
- **L'instantané** : c7 demandait `avant-835d5298f`, qui n'existait pas à 19 h 52, ni à luciepc ni
  au portable. La lib courante était encore 207701bb2 (le commit de format), et 835d5298f n'était
  pas posé. J'ai donc lu l'instantané le plus récent, `avant-207701bb2`. Ses fichiers de base datent
  du 11 novembre à 22 h 33 : rien n'y a été écrit depuis. Une lecture de `avant-835d5298f` se rejoue
  par la même commande quand il existera (voir « Rejouer »).
- **La lib qui lit** : `rag3db-build-835d5298f80b832377a7caa80ee58d242e3ece23-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF`
  (`librag3db.so` md5 4caf4df6d9b3), en **lecture seule** (`SystemConfig::readOnly`). La sonde écrit
  le build id lu par `CALL build_id()` en tête de son relevé.
- **Le programme** : `outils/sonde-orphelins.cpp`, joint à cette page (binaire md5 f65c5a70b673 à
  luciepc), joué sous `poste lourd`, le verrou des travaux lourds de luciepc.

## Comment la sonde compte

1. **Par table de nœuds T** : l'ensemble des décalages vivants (`MATCH (n:T) RETURN offset(id(n))`) et
   le plus grand. Le décalage est la place du nœud dans sa table.
2. **Par table de relations R, de X vers Y, dans chaque sens** :
   - au **sens avant**, ancrée sur la source :
     `MATCH (a:X)-[r:R]->(b) WHERE offset(id(a)) >= 0 RETURN offset(id(b)), label(b)` ;
   - au **sens arrière**, ancrée sur la destination :
     `MATCH (b:Y)<-[r:R]-(a) WHERE offset(id(b)) >= 0 RETURN offset(id(a)), label(a)`.

   Un bout est **absent** quand son décalage n'est pas celui d'un nœud vivant. Cela couvre le décalage
   au-delà du plus grand (la mesure proposée par le cœur) comme une place vide sous le plus grand.
   La sonde ne lit jamais une propriété d'un bout manquant.
3. **Le sens de chaque requête est prouvé par son plan** (`EXPLAIN`). Le planificateur choisit seul
   d'où il part. Une première version de la sonde, avec `MATCH (a:X)-[r:R]->(b)` sans filtre, partait
   de la destination pour certaines tables : elle ne pouvait donc pas voir une destination disparue.
   C'était aussi la forme que proposait le cœur. Le filtre sur le bout d'ancrage, et le motif écrit
   depuis ce bout, font partir le plan du bon côté. La sonde lit le plan de chaque requête et
   signale « SENS NON PROUVÉ » sinon.
4. **Les morceaux** : pour chaque table `T_Chunk` qui a sa table `T`,
   `MATCH (k:T_Chunk) OPTIONAL MATCH (p:T {_uuid: k._parent_uuid}) WITH k, p WHERE p IS NULL RETURN count(k)`.
   Les bases de Lucie n'ont pas de table ScopeContent : elles ne portent ni ScopeContent ni
   ScopeContent_Chunk.

## Les limites

- **Une relation dont les deux bouts manquent** échappe aux deux sens : aucun bout vivant ne permet
  de l'atteindre.
- **Une seule paire au sens non prouvé, dans chaque base : Symbol_CHUNKED_FROM** (Symbol_Chunk →
  Symbol), au sens avant. La table de nœuds Symbol_Chunk est vide, et une table vide ne peut pas
  servir d'ancrage. Le sens arrière, lui, est prouvé : depuis les 84 928 Symbol vivants de
  code-lourd (73 373 dans code-principal), il ne trouve aucune relation. Une relation de cette table
  ne pourrait donc échapper que si ses deux bouts manquaient : c'est la limite précédente.
- **Les places vides** (« places vides sous le maximal » dans les tables) ne sont pas des orphelins :
  aucune relation n'y aboutit. Ce sont des nœuds effacés. Les 103 places vides de Scope dans
  code-lourd correspondent aux 103 Scope de fichiers disparus qu'un `index confirm` efface par
  `DETACH DELETE` (vu dans la trace du 8 novembre). Les 128 places vides de Scope_Chunk sont leurs
  morceaux. C'est un rapprochement, je ne l'ai pas prouvé nœud par nœud.

## Les chiffres

### code-lourd

**Relations**, par table et par sens. « Lues » est le nombre de relations vues depuis le bout d'ancrage ; « bout absent » compte celles dont l'autre bout n'est pas un nœud vivant (au-delà du décalage maximal, ou dans un trou).

| table de relations | de → vers | lues au sens avant | destination absente | lues au sens arrière | source absente | sens prouvé |
|---|---|---:|---:|---:|---:|---|
| CONSUMED_BY | Scope → Scope | 151 744 | 0 | 151 744 | 0 | oui, les deux |
| CONSUMES | Scope → Scope | 151 744 | 0 | 151 744 | 0 | oui, les deux |
| DECORATES | Scope → Scope | 0 | 0 | 0 | 0 | oui, les deux |
| DEFINED_IN | Scope → File | 79 031 | 0 | 79 031 | 0 | oui, les deux |
| DEFINES | Scope → Symbol | 96 012 | 0 | 96 012 | 0 | oui, les deux |
| File_CHUNKED_FROM | File_Chunk → File | 7 278 | 0 | 7 278 | 0 | oui, les deux |
| HAS_PARENT | Scope → Scope | 42 625 | 0 | 42 625 | 0 | oui, les deux |
| IMPLEMENTS | Scope → Scope | 804 | 0 | 804 | 0 | oui, les deux |
| INHERITS_FROM | Scope → Scope | 658 | 0 | 658 | 0 | oui, les deux |
| LOCKS | Scope → Symbol | 546 | 0 | 546 | 0 | oui, les deux |
| Library_CHUNKED_FROM | Library_Chunk → Library | 464 | 0 | 464 | 0 | oui, les deux |
| MENTIONS | Scope → Symbol | 466 845 | 0 | 466 845 | 0 | oui, les deux |
| PARENT_OF | Scope → Scope | 42 625 | 0 | 42 625 | 0 | oui, les deux |
| QUERIES | Scope → Symbol | 20 093 | 0 | 20 093 | 0 | oui, les deux |
| REGISTERS | Scope → Symbol | 350 | 0 | 350 | 0 | oui, les deux |
| Scope_CHUNKED_FROM | Scope_Chunk → Scope | 130 201 | 0 | 130 201 | 0 | oui, les deux |
| Symbol_CHUNKED_FROM | Symbol_Chunk → Symbol | 0 | 0 | 0 | 0 | avant non prouvé (table de nœuds d'ancrage vide) |
| USES_LIBRARY | Scope → Library | 17 712 | 0 | 17 712 | 0 | oui, les deux |

**Morceaux**

| table de morceaux | morceaux | sans parent |
|---|---:|---:|
| Library_Chunk (parent Library) | 464 | 0 |
| Scope_Chunk (parent Scope) | 130 201 | 0 |
| File_Chunk (parent File) | 7 278 | 0 |
| Symbol_Chunk (parent Symbol) | 0 | 0 |

**Tables de nœuds**

| table de nœuds | nœuds vivants | décalage maximal | places vides sous le maximal |
|---|---:|---:|---:|
| _DataflowNodeState | 46 | 61 | 16 |
| Symbol | 84 928 | 84927 | 0 |
| _snapshot_aside | 103 | 102 | 0 |
| Library | 464 | 463 | 0 |
| _Org | 1 | 0 | 0 |
| _index_blobs | 0 | -1 | 0 |
| Library_Chunk | 464 | 463 | 0 |
| Scope_Chunk | 130 201 | 130328 | 128 |
| File | 7 278 | 7277 | 0 |
| _catalog_meta | 48 | 47 | 0 |
| _DataflowExecution | 22 | 21 | 0 |
| File_Chunk | 7 278 | 7277 | 0 |
| _Project | 1 | 0 | 0 |
| Symbol_Chunk | 0 | -1 | 0 |
| Scope | 82 855 | 82957 | 103 |

### code-principal

**Relations**, par table et par sens. « Lues » est le nombre de relations vues depuis le bout d'ancrage ; « bout absent » compte celles dont l'autre bout n'est pas un nœud vivant (au-delà du décalage maximal, ou dans un trou).

| table de relations | de → vers | lues au sens avant | destination absente | lues au sens arrière | source absente | sens prouvé |
|---|---|---:|---:|---:|---:|---|
| CONSUMED_BY | Scope → Scope | 101 753 | 0 | 101 753 | 0 | oui, les deux |
| CONSUMES | Scope → Scope | 101 753 | 0 | 101 753 | 0 | oui, les deux |
| DECORATES | Scope → Scope | 0 | 0 | 0 | 0 | oui, les deux |
| DEFINED_IN | Scope → File | 62 286 | 0 | 62 286 | 0 | oui, les deux |
| DEFINES | Scope → Symbol | 78 468 | 0 | 78 468 | 0 | oui, les deux |
| File_CHUNKED_FROM | File_Chunk → File | 6 314 | 0 | 6 314 | 0 | oui, les deux |
| HAS_PARENT | Scope → Scope | 34 147 | 0 | 34 147 | 0 | oui, les deux |
| IMPLEMENTS | Scope → Scope | 334 | 0 | 334 | 0 | oui, les deux |
| INHERITS_FROM | Scope → Scope | 601 | 0 | 601 | 0 | oui, les deux |
| LOCKS | Scope → Symbol | 436 | 0 | 436 | 0 | oui, les deux |
| Library_CHUNKED_FROM | Library_Chunk → Library | 398 | 0 | 398 | 0 | oui, les deux |
| MENTIONS | Scope → Symbol | 376 651 | 0 | 376 651 | 0 | oui, les deux |
| PARENT_OF | Scope → Scope | 34 147 | 0 | 34 147 | 0 | oui, les deux |
| QUERIES | Scope → Symbol | 20 210 | 0 | 20 210 | 0 | oui, les deux |
| REGISTERS | Scope → Symbol | 350 | 0 | 350 | 0 | oui, les deux |
| Scope_CHUNKED_FROM | Scope_Chunk → Scope | 104 997 | 0 | 104 997 | 0 | oui, les deux |
| Symbol_CHUNKED_FROM | Symbol_Chunk → Symbol | 0 | 0 | 0 | 0 | avant non prouvé (table de nœuds d'ancrage vide) |
| USES_LIBRARY | Scope → Library | 11 654 | 0 | 11 654 | 0 | oui, les deux |

**Morceaux**

| table de morceaux | morceaux | sans parent |
|---|---:|---:|
| Library_Chunk (parent Library) | 398 | 0 |
| Scope_Chunk (parent Scope) | 104 997 | 0 |
| File_Chunk (parent File) | 6 314 | 0 |
| Symbol_Chunk (parent Symbol) | 0 | 0 |

**Tables de nœuds**

| table de nœuds | nœuds vivants | décalage maximal | places vides sous le maximal |
|---|---:|---:|---:|
| _DataflowNodeState | 50 | 67 | 18 |
| Symbol | 73 373 | 73372 | 0 |
| _snapshot_aside | 102 | 101 | 0 |
| Library | 398 | 397 | 0 |
| _Org | 1 | 0 | 0 |
| _index_blobs | 0 | -1 | 0 |
| Library_Chunk | 398 | 397 | 0 |
| Scope_Chunk | 104 997 | 105123 | 127 |
| File | 6 314 | 6313 | 0 |
| _catalog_meta | 48 | 47 | 0 |
| _DataflowExecution | 23 | 22 | 0 |
| File_Chunk | 6 314 | 6313 | 0 |
| _Project | 1 | 0 | 0 |
| Symbol_Chunk | 0 | -1 | 0 |
| Scope | 65 914 | 66015 | 102 |

## Seconde sonde : les mots perdus du dictionnaire de lucivy (13 novembre, 20 h 30)

**Le défaut** (ticket de recherche
`docs/recherche/tickets/2026-11-13-un-index-lucivy-ecrit-avant-4-3-5-pendant-des-fusions-garde-ses-mots-perdus.md`,
branche `lucivy-4-3-5`). Avant lucivy 4.3.5, une fusion de segments lancée entre deux commits faisait
perdre au dictionnaire partagé les mots des segments écrits depuis le dernier commit. Un index
plein texte (`.fts`) touché garde ces pertes, et la montée de version ne les répare pas. Un mot
perdu se voit ainsi : le segment le désigne par un identifiant (dans son fichier `.gmap`) que le
dictionnaire du shard ne connaît pas (ni texte, ni métadonnée).

**La réponse : 0 identifiant inconnu dans les quatre `.fts`.**

| base | poste | identifiants lus | inconnus du dictionnaire | segments en attente |
|---|---|---:|---:|---:|
| code-lourd | luciepc | 6 945 348 | **0** | 0 |
| code-principal | luciepc | 5 732 393 | **0** | 0 |
| code-rag3db-depot | portable | 11 818 307 | **0** | 0 |
| memoire-longue | portable | 261 925 | **0** | 0 |

**Le témoin rouge.** lucivy m'a confié une copie de son index sauvé, à luciepc :
`~/.cache/rag3weaver-build/lucivy-index-rouge-pour-hotfixs`, identique à
`~/.cache/rag3weaver-build/lucivy-flou/index-rouge-60f7063`. Sur une copie neuve, la sonde compte
**31 862 identifiants inconnus sur 2 693 974** dans le champ `content`, exactement le chiffre de
lucivy. Elle en compte aussi 480 sur 26 493 dans le champ `path`. C'est le même défaut, confirmé
par lucivy : `build_schema` (`lucivy_core/src/handle.rs`) ignore l'option `indexed` pour un champ
texte, donc tout champ texte est indexé, avec sa part du dictionnaire partagé (`sfx_field_ids [1, 2]`
dans le `meta.json` de l'index rouge). Compter tous les champs texte est la bonne règle : les 0 des
bases de Lucie valent pour tous leurs champs.

**Ce qui a été lu.** Des copies neuves des `.fts` de l'instantané `avant-835d5298f` (luciepc à
20 h 06, portable à 20 h 07), prises hôtes arrêtés avec `fuser` vide. L'outil ouvre chaque shard
avec un writer, qui peut écrire : chaque lancement se fait sur une copie neuve, jamais sur un index
en service.

**L'outil.** Une variante du test de lucivy `diag_flou_ids_sans_texte`, nommée
`diag_ids_sans_texte_tous_champs`, dans `lucivy_core/tests/diag_flou_monotonie.rs`, sur la branche
lucivy `diag-mots-perdus-tous-champs` (a4e2d16, au-dessus de `diag-flou-monotonie` 9d7b6b1, soit
lucivy-core 4.3.4). Elle parcourt chaque table `Lucivy_*` d'un `.fts`, chaque `shard_*` et chaque
champ texte qui a un `.gmap`. Elle lit `pending_segments` dans le `meta.json` de chaque shard :
aucun n'en avait. Une copie de l'outil est jointe dans `outils/diag_flou_monotonie.rs`.

**Rejouer** (à luciepc ; le binaire est déjà dans `~/.cache/rag3db-tickets-notes/lucivy-sonde/`) :

```bash
ssh -T lucied@luciepc 'bash -s' <<'EOF'
X=~/.cache/rag3db-tickets-notes/lucivy-sonde/diag_flou_monotonie
I=~/.local/share/rag3weaver/instantanes/<instantané>; b=code-lourd
C=~/.cache/rag3db-locks/sonde-lucivy/$b-<étiquette neuve>
mkdir -p $C && cp -a --reflink=always $I/$b/$b.rag3db.fts $C/
cd ~/.cache/rag3weaver-build && DIAG_FTS=$C/$b.rag3db.fts ./poste lourd timeout 3600 $X diag_ids_sans_texte_tous_champs --ignored --nocapture > $C.txt 2>&1
grep -E 'PERDUS|^ATTENTE|^BILAN' $C.txt
EOF
```

Relevés : à luciepc dans `~/.cache/rag3db-locks/sonde-lucivy/{code-lourd-1,code-principal-1,temoin-rouge-1}.txt`,
et au portable dans `~/.cache/rag3db-locks/sonde-lucivy/{memoire-longue-1,code-rag3db-depot-1}.txt`.
Aucun `.fts` de Lucie n'a été reconstruit : la reconstruction est à son mot, et ces chiffres
disent qu'elle n'a rien à rattraper.

## Rejouer (la sonde des orphelines)

À luciepc, toujours sur une copie et jamais sur une base tenue par un hôte. Luciepc est en fish :
passer par `bash -s`.

```bash
ssh -T lucied@luciepc 'bash -s' <<'EOF'
N=~/.cache/rag3db-tickets-notes
M=~/.cache/rag3db-libs/rag3db-build-835d5298f80b832377a7caa80ee58d242e3ece23-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF
I=~/.local/share/rag3weaver/instantanes/<instantané>     # par exemple avant-835d5298f quand il existera
S=~/.cache/rag3db-locks/sonde-orphelins-<étiquette neuve>
c++ -std=c++20 -O1 $N/sonde-orphelins.cpp -I$M -L$M -Wl,-rpath,$M -lrag3db -o $N/sonde-orphelins
for b in code-lourd code-principal; do
  [ -z "$(fuser $I/$b/* 2>/dev/null)" ] || { echo "$b est ouverte"; continue; }
  mkdir -p $S/$b && cp -a --reflink=always $I/$b/. $S/$b/
  printf 'VECTOR\t%s/libvector.rag3db_extension\n' "$M" > $S/$b/$b.rag3db.extensions
  (cd ~/.cache/rag3weaver-build && ./poste lourd timeout 3600 $N/sonde-orphelins $S/$b/$b.rag3db > $S/$b.txt 2>&1); echo "$b code $?"
  grep -E 'ORPHELIN|NON PROUVÉ|^BILAN|^ERREUR' $S/$b.txt
done
EOF
```

Le code de sortie vaut :
- 0 : aucun orphelin, tous les sens prouvés ;
- 1 : des orphelins ;
- 2 : une requête a échoué, ou un sens n'est pas prouvé (lire la ligne).

Sur les bases de Lucie, on attend 2 à cause de Symbol_CHUNKED_FROM : voir « Les limites ».

Les relevés complets du 13 novembre sont à luciepc, dans
`~/.cache/rag3db-locks/sonde-orphelins-13nov/{code-lourd,code-principal}-3.txt`. Les fichiers `.txt`
et `-2.txt` y sont les deux premières versions, au sens non prouvé, à ne pas lire. Une copie est au
portable, dans `~/.cache/rag3db-locks/sonde-orphelins/lucie/`. Le témoin sur la base du cœur est dans
`~/.cache/rag3db-locks/sonde-orphelins/temoin-base-2.txt` au portable.

## Ce qui n'a pas été fait

Aucune réparation, aucune écriture sur une base de Lucie : la décision est à elle. Les copies restent
sur disque (environ 100 Mio à luciepc) et peuvent partir.
