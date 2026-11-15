# DdlEveryMinute meurt d'un SIGSEGV au troisième tour, pile perdue

- **État** : ouvert (15 novembre 2026, cœur c++)
- **Famille** : moteur
- **Gravité** : plantage (SIGSEGV du processus entier)
- **Atteignable en service** : à établir
- **Touche rag3weaver** : par le moteur (l'hôte meurt)
- **Signalé par** : cœur c++. Passe du lot du verrou de schéma (`alter-attend-les-ecrivains`).
- **Pour** : cœur c++

## Ce qu'on voit

`TwentyWriters.DdlEveryMinute`, tour 3 sur 3, joué à luciepc le 15 novembre sur `ed2b6ffe6`
(master `a2f9c9b65` plus le lot du verrou de schéma), bâti Release dans
`~/git_workspaces/rag3db-colonnes/build/moteur`. Le processus meurt d'un SIGSEGV à 09:33:29,
après le contrôle `index-creation-refuses-no-reader` (rouge connu) ; il porte le core
systemd-coredump du pid 1604744. Les tours 1 et 2 ont fini sans mourir.

**La pile est perdue.** Le départage qui a suivi a rebâti `concurrence_test` par-dessus le
binaire qui avait produit le core. `coredumpctl debug` ne donne que des adresses sans symboles.
Je n'attribue donc pas ce plantage au SIGSEGV du dictionnaire (ticket
`2026-11-14-un-point-de-reprise-automatique-meurt-dans-le-dictionnaire-d-une-colonne-de-chaines`) :
ce serait sans preuve.

## Ce qui est fait

Les tours suivants gardent une copie du binaire à côté de leur journal :
`~/.cache/rag3db-moteur-notes/alter7/concurrence_test.garde`, le binaire même qui tourne. Un
nouveau core aura sa pile.

## Pour fermer

Un SIGSEGV qui revient : lire sa pile, puis le rattacher au ticket du dictionnaire ou le décrire
ici. Aucun sur dix tours de suite : fermer en le disant.
