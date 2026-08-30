# Comment on saura que ça marche

Troisième et dernier document du cahier des charges. Celui-ci ne décrit pas ce
qu'il faut construire, mais **ce qui doit être vrai à la fin** — et comment le
vérifier sans se croire sur parole.

À écrire d'avance, parce qu'un critère de réussite inventé après coup s'aligne
toujours sur ce qu'on a réussi à faire.

## 1. L'invariant, testé sur du vrai

```rust
// Pour chaque fichier d'un corpus réel :
let mut couvert = 0;
let mut fin_precedente = 0;
for s in scopes_de_premier_niveau {
    assert_eq!(s.scope_start_byte, fin_precedente, "trou ou recouvrement");
    fin_precedente = s.scope_end_byte;
    couvert += s.scope_end_byte - s.scope_start_byte;
}
assert_eq!(fin_precedente, taille_du_fichier);
assert_eq!(couvert, taille_du_fichier);
```

**Sur trois corpus, et les trois comptent :**

1. **`src/` de ce dépôt** — 89 fichiers Rust bien formés. L'invariant doit
   tenir sans un seul passage `TexteNonCompris`.
2. **Des fichiers volontairement cassés** — une accolade en trop, un fichier
   Rust renommé en `.go`, un binaire renommé en `.rs`. L'invariant doit tenir
   *aussi*, et les passages doivent être marqués non compris.
3. **Des fichiers sans parseur** — un `.md`, un `.toml`, un `.sh`. Un scope,
   couvrant tout, `arbre_en_erreur: false`.

Le point 2 est le seul qui prouve quelque chose. Les deux autres passeraient
avec une implémentation naïve.

## 2. Les chiffres qu'on veut voir, et ceux qui trahiraient un défaut

Sur `src/` de ce dépôt, après implémentation :

| ce qu'on attend | ce que ça voudrait dire si c'était faux |
|---|---|
| Rust : **0** fichier en erreur | notre propre code ne parse pas — le parseur est cassé, pas le corpus |
| Rust : **> 90 %** d'octets en code | les scopes ratent des constructions entières (des `impl` ? des macros ?) |
| C++ (`extension/`) : le taux réel | c'est la mesure qu'on n'a jamais eue, et la raison de tout ce chantier |
| **0** scope vide | le découpage produit du bruit sur les lignes blanches |
| Nombre de scopes : **+ 20 à 60 %** | en dessous, on n'a rien ajouté ; bien au-dessus, on fragmente |

Ce dernier ordre de grandeur est une **hypothèse à confirmer**, pas une cible :
si le compte double, il faut comprendre pourquoi avant de se réjouir.

## 3. La recherche ne doit pas reculer

C'est le critère qui compte le plus, et le plus facile à oublier.

**Le banc existe** : `e2e_catalogue_gabarits::brique_2_quel_signal_ment`, trois
questions en français, avec les scores attendus — `product 3,85 · user 7,23 ·
conversation 7,30` depuis le correctif de l'issue 02.

**La mesure** : ingérer le même corpus avec la couverture activée, et refaire
passer les trois questions. Si les réponses reculent, le bruit est réel et il
faut peser. Si elles ne bougent pas, on n'a rien cassé.

Et un second banc à écrire : **une question dont la réponse est dans un passage
non compris**. C'est le seul moyen de vérifier que le gain existe, et pas
seulement l'absence de perte.

## 4. Ce qui reste ouvert, et qu'il ne faut pas trancher trop vite

- **La pondération par genre.** Ne pas l'implémenter avant d'avoir la mesure du
  §3. Un poids réglé à l'aveugle est plus difficile à retirer qu'à ajouter.
- **Le seuil de découpage** d'un passage non compris. Le `Chunker` a déjà ses
  réglages ; commencer par les mêmes que pour un document, et regarder.
- **Les fichiers binaires.** Un `.png` dans un dépôt ne doit pas devenir un
  scope de texte. Une détection — octet nul dans les premiers kilo-octets —
  suffit probablement, et il faut le décider explicitement plutôt que de le
  découvrir.
- **Le volume.** Combien de fichiers de ce dépôt sont aujourd'hui hors index ?
  À compter **avant** de commencer : c'est la seule façon de savoir si ce
  chantier ajoute 2 % ou 200 % au corpus.

## 5. Le seul critère qui vaut vraiment

Tout le reste peut être vert et le chantier raté. La question finale est
celle-ci :

> **Peut-on maintenant répondre à « quels langages servons-nous mal ? » en
> lisant une sortie, plutôt qu'en devinant ?**

Si oui, c'est réussi, même si les taux sont mauvais — parce qu'un mauvais taux
qu'on voit vaut mieux qu'un bon taux qu'on suppose. Si non, on a ajouté du code
sans ajouter de connaissance.
