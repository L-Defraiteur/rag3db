# Le shell perd des caractères d'une entrée par pipe

- **État** : ouvert (16 novembre 2026, hotfixs), vu en mesurant, pas encore cerné
- **Famille** : moteur
- **Gravité** : une instruction d'un script lu par le shell est exécutée tronquée, ou refusée. Ici le shell a refusé
  l'instruction et l'a dit, mais une instruction tronquée qui reste valide s'exécuterait sans un mot.
- **Atteignable en service** : par quiconque passe un script au shell sur son entrée (`rag3db base < script.cypher`)
- **Touche rag3weaver** : non
- **Pour** : hotfixs

## Ce qu'on voit

Le 16 nov., à luciepc, le shell lit sur un pipe un script de 401 lignes : `CREATE NODE TABLE …`, puis 200 fois
`CREATE (:T {id: i});` et `CHECKPOINT;`, puis un `count(*)`. Une fois par partie, et dans chacune des dix parties
(avec un shell bâti de 1688239b2 comme avec un shell bâti de 4676ce513), la même erreur sort :

```
Error: Parser exception: extraneous input 'CHECKPOIEATE' expecting {ALTER, ATTACH, BEGIN, CALL, CHECKPOINT, …
```

La fin de `CHECKPOINT;` et le début de la ligne suivante, `CREATE`, sont lus d'un seul tenant, et des caractères se
perdent au passage (`NT;\nCR`). Le `count(*)` final rend 199 au lieu de 200 : une validation n'a pas eu lieu.

## La piste, à vérifier

`linenoise.cpp` traite une entrée qui arrive en bloc comme un collage (`pastedInput`, `hasMoreData`), et suit alors
d'autres chemins pour Entrée et la tabulation. Il pourrait aussi y avoir une lecture bornée par un tampon, qui
coupe une ligne à sa limite. La même perte, au même endroit à chaque partie, désigne un compte ou une taille fixe.

## Pour fermer

1. Un témoin rouge : le même script passé au shell par un pipe, avec 200 lignes attendues au `count(*)`.
2. La cause dans la lecture de l'entrée, le correctif, puis le témoin vert.
