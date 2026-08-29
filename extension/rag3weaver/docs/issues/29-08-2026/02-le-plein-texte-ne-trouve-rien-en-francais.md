# Le plein texte ne rend rien sur une phrase française

**Trouvé le 29 août 2026**, en isolant les signaux
(`brique_2_quel_signal_ment`). Défaut **distinct** de
[l'issue 01](01-le-vecteur-du-moteur-ne-classe-pas.md), trouvé au même endroit.

## Le symptôme

Trois requêtes, signal `bm25` seul, sur des descriptions qui contiennent
littéralement les mots cherchés :

```
« vendre des articles avec un prix »   → (rien)
« de quoi savoir qui est connecté »    → (rien)
« suivre un échange entre personnes »  → (rien)
```

Alors que `product` dit *« son prix et sa devise »*, `user` dit *« sait qui se
connecte »*, `conversation` dit *« un fil de discussion entre plusieurs
personnes »*.

## La cause, probablement

Le moteur l'annonce lui-même dans `SearchMeta.warnings` :

```
fuzzy search ignores separators:
"de quoi savoir qui est connecté sur mon site"
is searched as "dequoisavoirquiestconnectésurmonsite"
```

La requête entière devient **un seul terme**, collé. Aucun document ne contient
ce mot, donc aucun résultat. Le mode par défaut (`BM25Mode::Contains`, flou)
convient à un identifiant — `merge_port_values` — et pas à une phrase.

## Ce que ça dit de plus

Le mécanisme **avertit** correctement, et personne ne lit l'avertissement : il
part dans `meta.warnings`, que ni le rendu ni l'agent ne montrent. Un
avertissement qui ne remonte pas est un avertissement qui n'existe pas — c'est
la même famille que les sept mécanismes construits et jamais branchés de cette
semaine.

## Pistes

- **Choisir le mode selon la requête** : une phrase (des espaces, plusieurs
  mots) veut `Parse` ou `ContainsSplit` ; un identifiant veut `Contains`. Le
  moteur a déjà les trois modes, et `e2e_phase0b` les mesure.
- **Ou remonter l'avertissement dans le rendu**, pour qu'un agent voie
  pourquoi il n'a rien trouvé et reformule. C'est la règle n° 3 du domaine de
  travail, appliquée à la recherche : distinguer « ça n'existe pas » de
  « je n'ai pas cherché ce que tu crois ».
