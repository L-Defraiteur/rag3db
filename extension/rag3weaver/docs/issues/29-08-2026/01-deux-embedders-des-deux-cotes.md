# Deux embedders des deux côtés — **résolu, et c'est un piège du moteur**

**Ouvert et fermé le 29 août 2026.** Reproduit puis levé par
`tests/e2e_catalogue_gabarits.rs`, briques 1 à 3.

## Ce que je croyais avoir trouvé

Le cosinus calculé à la main était juste 3/3 ; le même calcul par
`Catalog::search` était faux 2/3, avec `user` premier aux trois questions et
des scores autour de zéro. J'ai écrit ici que le chemin vectoriel du moteur ne
classait pas.

**C'était faux.** Le moteur classe parfaitement. Après correction, ses scores
sont **exactement** ceux du cosinus nu, au chiffre près :

```
« vendre des articles avec un prix »  → product 0.6169 · conversation 0.4570 · user 0.3846
« de quoi savoir qui est connecté »   → user    0.6007 · conversation 0.4724 · product 0.4383
« suivre un échange entre personnes » → conversation 0.6623 · user 0.4994 · product 0.4444
```

## La cause

Le montage embarquait les **documents** avec BGE-M3 et les **requêtes** avec un
`HashEmbedder` — deux espaces vectoriels sans rapport.

```rust
let mut catalog = Catalog::new(conn, Box::new(HashEmbedder::new(1024)), config);
catalog.set_dual_embedder(bge_m3);   // ← n'indexe QUE les documents
```

`Catalog::search` embarque la requête avec l'embedder **du catalogue**
(`src/catalog.rs:3812`, `embed_query(self.embedder.as_ref(), …)`), tandis que
`set_dual_embedder` ne change que ce qui indexe. Le nom ne le dit pas.

## Pourquoi ça reste une issue

**Le moteur ne dit rien.** Deux embedders incompatibles produisent des scores
plausibles — pas d'erreur, pas d'avertissement, juste un classement qui ne veut
rien dire. C'est exactement la famille de défauts qu'on passe la semaine à
débusquer : *ce dont l'absence ne se voit pas*.

Trois pistes, par ordre de coût :

1. **Refuser au montage.** `set_dual_embedder` sait comparer sa dimension à
   celle de l'embedder du catalogue ; il pourrait aussi comparer une empreinte
   de modèle (`Embedder::name()` existe-t-il ? sinon l'ajouter). Deux
   embedders différents des deux côtés est presque toujours une erreur.
2. **Ou faire de `set_dual_embedder` ce que son nom promet** : qu'il serve
   aussi aux requêtes. C'est ce qu'un appelant attend.
3. **Ou l'écrire dans la fiche** de `Catalog::new` — le moins cher, le moins
   sûr.

Et **le montage fautif est copié ailleurs** :
`tests/e2e_conversation_a_plusieurs.rs` fait exactement la même chose. Il ne
s'en est pas aperçu parce que ses agents cherchent par le chemin **dataflow**
(`VectorSearchNode` prend l'embedder du registre de services, BGE-M3 des deux
côtés), pas par `Catalog::search`. Les sondes du témoin, elles, passent par le
graphe-outil — donc elles étaient justes. Mais la ligne est là, et elle
piégera le prochain.

## Ce que la méthode a donné

La dichotomie de Lucie — *essayer chaque brique séparément* — a tenu :

| brique | mesure | verdict |
|---|---|---|
| 1. cosinus nu | 3/3 | l'embedder est juste |
| 2. chaque signal isolé | vecteur 1/3, plein texte 0/3 | le défaut est en aval de l'embedder |
| 3. texte réellement découpé | titre + description exacts | l'indexation est juste |
| — | `catalog.rs:3812` | **la requête n'utilise pas le même embedder** |

Sans la brique 3, j'aurais cherché dans la fusion et la normalisation — deux
hypothèses que la lecture avait déjà écartées (`fuse_signals` rend une source
unique **telle quelle**, sans normaliser).
