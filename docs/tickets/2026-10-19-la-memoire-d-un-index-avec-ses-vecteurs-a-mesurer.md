# La mémoire d'un index avec ses vecteurs : à mesurer

- **État** : ouvert (19 octobre 2026, G et le banc). C'est la suite du
  [ticket fermé de la mémoire d'un gros index](closed/2026-10-18-l-hote-ne-rend-pas-la-memoire-d-un-gros-index.md),
  dont le correctif n'est prouvé que pour la phase plein texte et relations.
- **Gravité** : mémoire. Rien n'est perdu ni faux.
- **Atteignable en service** : oui, après tout index qui paie sa dette de vecteurs (le rattrapage du
  12 octobre, qui avait laissé 10 Go dans l'échange, en était un).
- **Touche rag3weaver** : oui (l'hôte du démon).

## Ce qui manque

Le correctif (`malloc_trim(0)` en fin d'index, à la fermeture d'un backend et au repos, plus
`MALLOC_ARENA_MAX` dans l'unité) a été mesuré sans service d'embarquement : l'index faisait le plein
texte et les relations, les vecteurs restaient en dette. Les arènes et les tampons de
l'embarquement (les lots envoyés aux services, les réponses, les écritures de vecteurs) peuvent
garder autre chose, que `malloc_trim` ne rend pas.

## Le témoin à jouer

Le protocole du banc, comme au ticket fermé, avec les services d'embarquement de luciepc :
1. une passe A (sans correctif) et une passe B (avec), chacune seule, sur une base neuve ;
2. le corpus rag3db à `c6bb094fb`, tampon de 2 Gio, régime GPU doux
   (`RAG3WEAVER_EMBED_CHAR_BUDGET=4096`, `RAG3WEAVER_GPU_DUTY=70`) ;
3. T0 à T4 relevés par `sonde-hote.sh`, en RssAnon + VmSwap (luciepc envoie l'anonyme inactif en
   échange zram dès T0) ;
4. quand les services sont libres, sans déranger codeparsers ni l'hôte de luciepc.

Le pilote est `mesure-hote.py` (scratchpad de G, copie dans `~/.cache/rag3weaver-build/` sur
luciepc) : il suffit de ne plus retirer `RAG3WEAVER_EMBED*` de l'environnement de l'hôte.

## Pour le fermer

B tient les mêmes bornes que la phase plein texte (T3 ≤ T0 + tampon + 500 Mo, T4 ≤ T0 + 500 Mo),
et A les manque. Sinon, chercher ce que l'embarquement garde. T4 se relève comme au ticket fermé
(60 s après la fermeture du backend, après le rendu de la fermeture) : en plein texte, la marge
de B n'y était que de 39 Mio.
