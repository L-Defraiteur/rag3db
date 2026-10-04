# Le plein texte hors de la base — le protocole, avant le code

4 octobre 2026 — session embarquements, temps 2 de la demande de Lucie (« on
peut essayer un mode, pour l'instant »). Un **mode déclaré** ; le défaut
reste `BlobBacked`. Rien n'est codé. Générique : une entité quelconque, pas
le code. Le mode sans disque local (`BlobBacked`) reste disponible.

## Le principe

Les fichiers d'index de chaque entité vivent dans `<base>.fts/` ; lucivy les
lit en place. En base, une ligne de méta par index : **la marque de
génération** — le numéro de génération lucivy (le `opstamp` de son commit)
que les lignes validées couvrent. Elle s'écrit **dans la même transaction**
que les lignes du paquet.

## 1. Les `fsync` : chez nous, sans toucher à lucivy

lucivy expose ce qu'il faut : le trait public `ShardStorage`
(`lucivy_core::sharded_handle`) et `LucivyHandle::create/open(dir: impl
Directory)`, avec le trait `Directory` de `ld-lucivy`. rag3weaver écrit donc
**son propre répertoire** — c'est le motif que lucivy suit déjà dans
`BlobDirectory`, qui écrit son cache local « sans fsync » :

- `FtsDirectory` enveloppe le répertoire natif de lucivy pour les lectures ;
  `open_write` et `atomic_write` écrivent **sans `fsync`** (fichier
  temporaire puis renommage pour `atomic_write`), et notent chaque chemin
  écrit depuis la dernière génération ;
- `sync_generation()` synchronise ces fichiers puis les dossiers qui les
  contiennent, **une fois par génération** au lieu d'une fois par fichier ;
- `delete` est **différé** jusqu'à la validation de la génération suivante
  (voir 2) ;
- `FtsShardStorage` est le `ShardStorage` qui ouvre les morceaux sur ce
  répertoire.

**Rien à changer dans lucivy.** Une option de `MmapDirectory` (« pas de
`fsync` par fichier ») ferait la même chose plus simplement ; ce n'est pas
nécessaire, et c'est à Lucie de dire si elle la veut dans son dépôt.

## 2. Ce qu'on synchronise, dans quel ordre, et l'arrêt entre chaque étape

À chaque validation (tous les K paquets, ou à chaque écriture hors premier
index) :

1. lucivy valide : il écrit ses segments et remplace `meta.json` par
   renommage — sans `fsync` ;
2. `sync_generation()` : `fsync` des fichiers écrits, puis des dossiers (le
   renommage de `meta.json` n'est durable qu'avec son dossier) ;
3. la transaction de la base valide les lignes **et** la marque N ;
4. les suppressions différées de la génération N−1 sont faites.

| Arrêt après… | État au redémarrage | Ce qu'on fait |
|---|---|---|
| 1 (ou pendant) | lignes et marque à N−1 ; `meta.json` à N ou N−1, fichiers N peut-être incomplets | les fichiers de N−1 sont encore là (suppressions différées) : on rouvre lucivy sur le `meta.json` de N−1, gardé à côté (`meta.N-1.json`) ; les fichiers de N non référencés sont supprimés |
| 2 | idem, fichiers N complets mais non référencés par la base | idem : N est jeté |
| 3 | lignes et marque à N ; fichiers N durables | rien : cohérent |
| 4 | idem, des fichiers N−1 en trop | on les supprime à l'ouverture |

Si la génération de la marque est introuvable sur le disque (dossier perdu,
copié à moitié, effacé), l'index est **rebâti depuis les lignes** — jamais
un plein texte vide servi en silence (3).

**Second lecteur** : les fichiers ne sont jamais modifiés en place, et
`meta.json` change par renommage. Un autre processus voit donc un commit
lucivy entier, éventuellement plus récent que les lignes qu'il lit : les
résultats passent déjà par les lignes de la base (résolution des morceaux),
un résultat sans ligne tombe.

## 3. Le nettoyage, et le rebâti

- Le dossier suit la base : créé avec elle ; supprimé avec elle par
  rag3weaver (`drop_storage` de lucivy existe) ; une copie de base copie le
  dossier, et la fonction de copie du crate le fait.
- Dossier absent, ou génération de la marque introuvable : l'état d'index
  de l'entité passe à « mots : en cours » (`IndexState.text = Running`), la
  recherche le dit, et le plein texte est rebâti depuis les lignes en tâche
  de fond. Jamais de vide muet.

## 4. La politique de fusion, réglage à part

Fait mesuré (passe en base, 4 octobre) : **1 951 Mo écrits pour un index qui
fait 687 Mo** une fois fini, soit ×2,8 — les fusions de segments et les
générations. lucivy a déjà `MergePolicy`, `NoMergePolicy`,
`IndexWriter::set_merge_policy` et `merge(segment_ids)`. Réglage proposé :
aucune fusion pendant un premier index, une fusion à la fin. Mesuré **après**
le protocole, pas mêlé à lui.

## 5. Le report de la poussée des blobs

Le report de la session de l'arbre principal ne concerne que `BlobBacked` :
en mode fichiers il n'y a plus de blobs à pousser. Il garde son utilité tant
que `BlobBacked` est le défaut.

## Les écarts aux moteurs établis

- **PostgreSQL** synchronise les fichiers de données au point de reprise et
  rejoue son journal ; nous n'avons pas de journal du plein texte : une
  génération non validée est jetée, et au pire l'index est **rebâti depuis
  les lignes** — c'est l'écart principal, accepté parce que les lignes sont
  la source et que le rebâti est un « en cours » visible.
- **Tantivy** fait un `fsync` par fichier et par dossier à chaque commit ;
  nous un seul par génération, parce que la durabilité vient de la marque en
  base, pas du commit lucivy.
- **Neo4j** rebâtit un index Lucene incohérent au démarrage, comme nous ;
  il garde en plus un journal pour rejouer les dernières transactions dans
  l'index, que nous n'avons pas.

## La preuve exigée

Un test d'arrêt brutal (processus tué dans la portée de la base) à chacune
des quatre étapes de 2, puis un processus neuf : la base s'ouvre, la marque
et les fichiers concordent ou l'index est rebâti, et **les comptes du plein
texte sont justes** (chaque ligne trouvée par un mot de son texte, aucun
résultat sans ligne).
