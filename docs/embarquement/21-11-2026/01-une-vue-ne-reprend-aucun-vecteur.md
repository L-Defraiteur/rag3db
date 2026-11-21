# Embarquement : une vue ne reprend aucun vecteur de la base, et pourquoi

Demande de c7 (21 novembre) : ED a mesuré l'indexation d'une vue (412 fichiers, 613 s) ; `derive_embed` en prend 407,
avec `dense_known=0` et 14 349 contenus insérés, alors qu'une vue partage l'essentiel de son code avec la base du dépôt.
ED prend l'écriture (`write_vectors`) ; ici, la reprise.

## La cause : un manque de conception

- **La seule reprise qui existe** est celle des vecteurs mis de côté avant purge (`catalog/aside.rs`, `KnownVectors`) :
  les lignes d'une même entité qui reparaissent, retrouvées **par l'uuid du morceau**, sous le même modèle (`slug`).
  `EmbedNode` (`dataflow/record_nodes.rs`) les reprend quand l'empreinte du texte (`_text_hash`) et la dimension
  concordent.
- **Une vue rend des contenus neufs** (`derive` → `derive_insert` → `derive_chunk`) : morceaux neufs, uuid neufs. Rien
  ne cherche un vecteur déjà calculé **par l'empreinte du texte** parmi les morceaux de la base. `dense_known` vaut donc 0
  par construction. Le `copied=20677` de `derive_chunk_link` compte des liens, pas des vecteurs.
- **L'empreinte permettrait de les retrouver** : `_text_hash` est l'empreinte du seul texte du morceau
  (`content_hash(chunk.text)`), sans en-tête ni chemin. Un même passage de code, découpé pareil dans la vue et dans la
  base, porte la même empreinte.
- **Dans la mesure d'ED, rien n'était reprenable de toute façon** : elle tourne sous `HashEmbedder(768)` (colonne
  `embedding__hashembedder`), et la base sous un autre modèle n'offre pas de vecteurs de ce modèle. Un vecteur d'un
  autre modèle ne se reprend jamais.

## Le gain, mesuré par ED

Sonde `probe_reuse_by_text_hash` d'ED, le 21 novembre (copies `mesures/ed-sonde-vecteurs-20261121-102958`, source, et
`mesures/ed-sonde-deux-passes-20261121-091416`, mesurée) :

- **La vue** : 20 677 morceaux (20 639 textes distincts), dont **15 301 portent un texte que la base embarque déjà sous
  granite, soit 74,0 %**.
- **La base elle-même** : 161 873 morceaux avec un vecteur granite, dont 54 750 portent l'un de 26 063 textes répétés,
  soit **environ 28 700 vecteurs recalculés pour rien (18 %)**. Un repassage profite aussi de la reprise.
- **Ce que la reprise n'évite pas : l'écriture.** Un vecteur repris s'écrit comme un vecteur calculé, 36,6 ms par
  vecteur sous HNSW (mesure d'ED), contre 0,21 ms pour lire ceux qui le portent. Ne plus écrire, c'est-à-dire partager le
  morceau au lieu de le recopier, sauverait de l'ordre de 9 minutes sur 10 pour une telle vue. C'est le lot (b), du
  domaine d'ED, conforme à la conception de Lucie du 6 novembre (« un index par dépôt, des vues par worktree »).

## La forme proposée

1. **La reprise par empreinte**, dans `EmbedNode`, après les filtres actuels : pour les morceaux denses qui restent à
   faire, **une requête par passe** cherche, dans les mêmes tables de morceaux, des morceaux qui portent la même
   `_text_hash` et un vecteur **valide pour le modèle courant**, c'est-à-dire dont la marque de la colonne du modèle
   vaut leur `_text_hash`. Un vecteur par empreinte, de la bonne dimension, se pose comme un vecteur connu
   (`embed_set` : vecteur et marque ensemble).
2. **Le même modèle seulement** : la colonne se résout par l'entrée du modèle (`VectorStorage::resolve`), comme à
   l'écriture ; un autre modèle a une autre colonne.
3. **Le creux et le dual** ne se reprennent pas (le creux ne se relit pas de l'index), comme aujourd'hui.
4. **Un seul parcours par paquet** : `MATCH (n:T) WHERE n._text_hash IN $hashes AND n.<marque> = n._text_hash AND
   n.<colonne> IS NOT NULL RETURN n._text_hash, n.<colonne>`, les empreintes en paramètre, en paquets de 20 000 au plus.
5. **Un compteur** : `dense_reused`, à côté de `dense_known`, au journal de profil.

## Les témoins, rouges d'abord

1. Une base dont un morceau porte déjà un vecteur ; une seconde entité, un contenu neuf au même texte : son morceau
   reprend le vecteur, sans appel au modèle (`dense_reused` = 1, embarqueur factice qui compte ses appels).
2. Le même cas sous un autre modèle : rien n'est repris.
3. Un texte changé d'un caractère : rien n'est repris.

## Le plan, prouvé sur la vraie base (sonde d'ED, 21 novembre)

Sur la copie source (161 873 morceaux, `ScopeContent_Chunk`), en lecture seule, release :

| Forme | 200 empreintes | 20 000 empreintes | Plan |
|---|---|---|---|
| `UNWIND $items … WHERE n._text_hash = item.hash` | 4,8 à 8,0 s | 16,4 à 33,6 s | produit cartésien |
| `WHERE n._text_hash IN $hashes`, marque comprise | 0,20 s | **1,26 s** (27 358 lignes) | **un seul parcours** |

- **La marque d'un modèle nommé** est `_embed_hash__<slug>` (`VectorStorage::resolve`, stockage `Suffixed`).
  `_embed_hash` est celle du stockage d'avant. Une copie peut porter les deux colonnes : c'est la première qui juge.
- **La condition sur la marque ne coûte rien** (1,26 s contre 1,39 s sans elle, sous la même charge), et rend les mêmes
  lignes sur une base à jour. Elle protège d'un vecteur périmé : un morceau dont le texte a changé et qui attend son
  réembarquement ne cède pas son vieux vecteur.
- **Une empreinte répétée rend plusieurs lignes** : le premier vecteur est gardé.

## Ce qui est livré

Le lot `reuse-by-hash` : `SchemaDialect::embed_find_by_hash` (rag3db ; un autre dialecte ne reprend rien),
`EmbedNode` qui pose les vecteurs repris par `write_vectors` (vecteur et marque ensemble), et le compteur `dense_reused`.
Le témoin, `catalog::reuse_by_text_hash_tests`, a été vu rouge (le corps déjà embarqué repartait au modèle), puis vert.
**Postgres** ne reprend pas encore : son dialecte rend `None`, à écrire au besoin.
