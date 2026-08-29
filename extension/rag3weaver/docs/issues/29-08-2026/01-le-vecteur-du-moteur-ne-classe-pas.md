# Le chemin vectoriel du moteur ne classe pas

**Trouvé le 29 août 2026.** Reproduit par
`tests/e2e_catalogue_gabarits.rs::brique_2_quel_signal_ment`.

## Le symptôme

Trois questions, trois gabarits d'entités (`user`, `product`, `conversation`),
chacune décrivant sans ambiguïté l'un d'eux. Le même embedder des deux côtés.

**Le cosinus calculé à la main est juste, 3 sur 3, et nettement :**

```
« vendre des articles avec un prix »  → product 0.6169 · conversation 0.4570 · user 0.3846
« de quoi savoir qui est connecté »   → user    0.6007 · conversation 0.4724 · product 0.4383
« suivre un échange entre personnes » → conversation 0.6623 · user 0.4994 · product 0.4444
```

**Le même calcul par `Catalog::search`, signal `vector` seul, est faux 2 sur 3 :**

```
« vendre des articles avec un prix »  → user 0.0401 · conversation -0.0200 · product -0.0585
« de quoi savoir qui est connecté »   → user 0.0681 · product      -0.0060 · conversation -0.0631
« suivre un échange entre personnes » → user 0.0439 · product      -0.0131 · conversation -0.0219
```

## Ce qui est établi

- **Ce n'est pas l'embedder.** Brique 1 (`brique_1_le_cosinus_nu_dit_il_la_verite`)
  le mesure séparément : 3/3.
- **Ce n'est pas le cross-encoder.** Aucun reranker n'est enregistré dans ce
  montage, et `reranked_count` vaut 0.
- **Ce n'est pas le filtre.** La même question sans filtre de famille rend le
  même ordre : `user` premier à 0,0401.
- **Ce n'est pas le catalogue de gabarits.** Il indexe (10/10), les facettes
  sont exactes (`category=auth` rend exactement `user`), la pose et le motif
  sont vérifiés.

## Les deux indices qui devraient suffire

**`user` sort premier aux trois questions**, toujours positif, les deux autres
toujours négatifs. Un classement qui ne dépend pas de la question n'en est pas
un : c'est un ordre fixe déguisé.

**L'échelle a changé.** Le cosinus vit entre 0,38 et 0,66 ; le moteur rend 0,04
à −0,06. Ce ne sont pas les mêmes nombres, donc pas le même calcul — quelque
chose transforme ou remplace la similarité entre le stockage et le rendu.

## Où regarder

Par ordre de vraisemblance, et chacun se vérifie sans deviner :

1. **Ce qui est écrit en base.** Lire les vecteurs stockés et les comparer à
   ce que l'embedder rend pour le même texte. Si l'ordre des chunks ou le
   préfixe de titre (`title_max_chars`, 256 caractères) change le texte
   embarqué, l'écart est là — et il expliquerait l'échelle **et** l'ordre fixe
   d'un coup.
2. **`search_vector` / `resolve_vector_chunks`** dans
   `src/dataflow/generic_search_nodes.rs` et le chemin `Catalog::search`.
3. **La normalisation** : un cosinus sur des vecteurs non normalisés n'est pas
   un cosinus.

## Ce que ça bloque

La thèse du [doc 08](../../vision_roadmap_08_2026/08-des-catalogues-de-gabarits.md) :
« un agent trouve ses capacités avec les moyens qu'il emploie pour trouver un
document ». Les facettes marchent, le sens non — donc un agent trouvera un
gabarit s'il connaît sa catégorie, pas s'il décrit son besoin.
