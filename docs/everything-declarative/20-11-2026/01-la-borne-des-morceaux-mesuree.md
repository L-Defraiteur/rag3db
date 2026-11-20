# La borne des morceaux, mesurée avant d'écrire

20 novembre 2026, 03 h 40, session everything declarative. Le lot des morceaux à la fenêtre de l'embarqueur
(ticket d'embarquement `2026-11-20-un-quart-des-morceaux-de-code-depasse-la-fenetre-de-granite.md`).

## La forme demandée

c7 l'a confirmée cette nuit :
- la borne d'un morceau entier (cœur et contexte) se déduit de la déclaration de l'embarqueur : fenêtre × marge ÷ ratio
  de jetons par caractère, déclaré par modèle ;
- elle est tenue par la base et entre dans la version de l'analyse ;
- le témoin rouge : un corpus découpé pour granite n'a aucun morceau au-dessus de sa fenêtre, compté par le vrai
  tokenizer.

## Ce que dit la mesure

Le banc de mesure : des paquets de lignes entières sous une borne de B octets, glissants de 10 lignes. Ils sont comptés
par le tokenizer de granite-278m (`~/.cache/rag3weaver/granite-278m/tokenizer.json`), jetons spéciaux compris. Le script
est dans la page, plus bas.

| corpus | B | paquets | max jetons | > 512 | jetons/caractère : médiane | p99 | p99,9 | max |
|---|---|---|---|---|---|---|---|---|
| rag3weaver `src/*.rs` | 1 024 | 24 377 | 507 | 0 | 0,341 | 0,441 | 0,472 | 0,523 |
| rag3weaver `src/*.rs` | 1 100 | 24 377 | 559 | 7 | 0,341 | 0,439 | 0,469 | 0,523 |
| rag3db `*.cpp` | 921 | 32 000 | 646 | 86 | 0,351 | 0,519 | 0,648 | 0,719 |
| rag3db `*.cpp` | 1 024 | 32 000 | 719 | 295 | 0,351 | 0,517 | 0,646 | 0,720 |
| rag3db `*.h` | 921 | 31 620 | 708 | 677 | 0,333 | 0,699 | 0,718 | 0,798 |
| rag3db `*.h` | 1 024 | 31 620 | 768 | 773 | 0,333 | 0,699 | 0,718 | 0,799 |
| rag3db `*.py` | 921 | 29 861 | 633 | 12 | 0,304 | 0,473 | 0,545 | 0,712 |
| rag3db `*.ts` | 1 024 | 146 | 396 | 0 | 0,322 | 0,448 | 0,464 | 0,464 |

**Ce qu'on en tire.**
- Le ratio moyen (0,45 mesuré par embarquement, 0,34 en médiane ici) tient sur le Rust de rag3weaver : à 1 024 octets,
  le pire paquet fait 507 jetons.
- Il ne tient pas sur le moteur. Les en-têtes C++ ont des tables denses (identifiants, nombres, symboles) : 1 % des
  paquets passe 0,7 jeton par caractère.
- Une borne qui garantit la fenêtre par le ratio seul se calcule sur le pire ratio : 512 × 0,9 ÷ 0,8 ≈ 576 octets. C'est
  deux fois plus de morceaux pour tout le code, à cause de 1 % des paquets.

**Donc la promesse « aucun morceau au-dessus de la fenêtre » ne se tient pas par un ratio déclaré.**

## Les deux formes qui la tiennent

1. **Le découpage compte les jetons**, avec le tokenizer de l'embarqueur le plus petit de la base. La borne en octets
   (fenêtre × marge ÷ ratio médian) reste le premier découpage, et elle est rapide. Puis un morceau qui dépasse la
   fenêtre se resserre : le contexte se rogne d'abord, puis le cœur se coupe à la ligne, jusqu'à tenir. Le tokenizer est
   une propriété du modèle, pas du service joint : la version de l'analyse reste déterministe, puisqu'elle contient le
   nom du modèle. Le coût : une tokenisation par morceau à l'index (le crate `tokenizers`, en Rust). Le crate est une
   dépendance optionnelle : sans lui, le découpage retombe sur la borne en octets et le dit au journal d'index.
2. **La borne en octets seule, par le ratio médian**, et un résidu déclaré. Environ 2 % des morceaux du moteur
   dépassent, et le journal d'index les compte par le compteur `truncated_texts` du service. Le témoin promet alors un
   plafond de résidu, pas zéro.

Je propose la forme 1. C'est la seule qui tient la promesse du ticket, et la borne en octets en reste le premier étage.

## Ce qui ne change pas d'une forme à l'autre

- La borne vaut pour le morceau entier, cœur et contexte. La fusion d'une queue courte avec le cœur précédent passe
  sous la même borne : c'est le second endroit où un morceau dépasse aujourd'hui (`chunk_lines`, phase 1).
- Le plus petit embarqueur déclaré pour la base fait la borne. Elle est tenue en méta et entre dans la version de
  l'analyse.
- bge-m3 seul garde 1 500 : sa fenêtre de 8 192 ne contraint rien.

## Le script

`mesure-des-paquets-en-jetons.py`, à côté de cette page : `uvx --offline --with tokenizers python -I mesure-des-paquets-en-jetons.py <racine> <bornes séparées par des virgules> <motif>`, par exemple `… ~/git_workspaces/rag3db 921,1024 "*.h"`.
