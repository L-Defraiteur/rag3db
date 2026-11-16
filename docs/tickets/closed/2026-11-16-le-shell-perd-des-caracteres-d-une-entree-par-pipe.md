# Le shell perd des caractères d'une entrée par pipe

- **État** : fermé (16 novembre 2026, hotfixs) : `getCursorPosition` ne questionne plus qu'un terminal (ea2e4bd34, branche `shell-pipe-input-keeps-bytes`).
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

## Fermeture (16 nov.)

**La cause.** Pour afficher un tableau, le shell demande la largeur de l'écran (`getColumns`,
`embedded_shell.cpp:1154` et `:1165`). Quand sa sortie n'est pas un terminal, `ioctl(TIOCGWINSZ)` échoue, et
`getColumns` se replie sur `getCursorPosition`. Celle-ci écrit `ESC[6n`, puis lit la réponse sur l'entrée brute,
octet par octet, jusqu'au premier « R ». Sous un pipe, cette « réponse » est le script lui-même : les octets que
stdio n'a pas encore pris dans son tampon de 4096, c'est-à-dire ceux qui suivent les 4096 premiers, jusqu'au premier
« R ». Dans le script de la mesure, ce sont `NT;\nCR`, à l'octet 4096 exactement, d'où `CHECKPOIEATE`. Le défaut ne
frappe qu'une fois : l'appel suivant trouve le pipe vidé dans le tampon de stdio et se replie sur 80 colonnes.

**Le témoin** `test_shell_basics.py::test_piped_input_past_4096_bytes_loses_nothing` passe 400 `RETURN k AS v;`
(plus de 4096 octets) par `ShellTest().run()`, entrée et sortie en pipes. Il est rouge 3 fois sur 3 sur le shell de
master : une instruction est lue coupée.

**Le correctif** : `getCursorPosition` rend -1 si l'entrée ou la sortie n'est pas un terminal, et `getColumns` comme
`getRows` se replient comme sur un échec.

**La passe** à luciepc, avec un shell bâti de ea2e4bd34 (sha256 f9932e66…) : le témoin vert 3 fois sur 3, la suite du
shell entière 246 sur 246.
