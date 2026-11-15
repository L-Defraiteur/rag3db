# Une passe confinée reçoit un SIGTERM sans cause dite, deux à trois minutes après le début d'une grosse analyse

- **État** : fait (15 novembre 2026, branche confineur-enveloppe d86628f55) : au signal, le confineur lit memory.peak et memory.events de la portée qui porte le plafond, écrit `enveloppe` au verdict et dit « enveloppe mémoire crevée (N Gio, plafond M Gio, k oom_kill dans <portée>) » ; trois témoins rouges d'abord, dont une vraie portée systemd-run ; test_confiner_validation 24 tests, aucun sauté
- **Famille** : produit (outillage : le confineur et `poste`)
- **Gravité** : mesure empêchée. Une passe s'arrête sans verdict, et rien ne dit pourquoi.
- **Atteignable en service** : non (les passes et les bancs, sous `poste lourd` et `confiner_validation.py`)
- **Touche rag3weaver** : oui (`scripts/confiner_validation.py`, et `poste` au cache)
- **Pour** : session codeparsers ; G (optimiseur) pour `poste`
- **Signalé par** : session codeparsers, au banc des scopes sans `DEFINED_IN` sur l'arbre entier du dépôt

## Ce qu'on voit

Trois passes, sous `poste lourd` (`POSTE_MEM_MAX=32G`) puis `confiner_validation.py lancer --delai 7200`,
détachées de l'outil (`setsid nohup`), chacune une analyse de l'arbre entier du dépôt (`third_party`
compris) par `code::analyze_in_project_avec`, en `--release` :

| preuves | lancée | arrêtée | forme |
|---|---|---|---|
| `validations/validation-gdn666gf` | 16 h 51 | 16 h 53 min 57 s | un seul appel |
| `validations/validation-91soylyj` | 16 h 54 | 16 h 59 min 16 s | par lots de 300 fichiers |
| `validations/validation-avgsjnqj` | 17 h 19 | 17 h 23 min 40 s | lots de 300, porte d'admission de l'index, 1 Mo au plus |

Chaque fois : le bâti finit, le test démarre, « has been running for over 60 seconds », puis le verdict du
confineur `"erreur": "signal reçu : 15"`, deux à trois minutes après le début du test. La même sonde sur
`extension/rag3weaver/src` seul (290 fichiers) va au bout.

Ce qui est exclu : le délai du confineur (7 200 s, loin d'être atteint) ; le délai de l'outil qui lance la
commande (la passe en est détachée) ; un arrêt écrit dans `poste` (aucun délai pour un lourd). Le journal
systemd ne garde aucune ligne des portées (`run-p…scope`), ni d'arrêt pour mémoire.

## La cause, confirmée

**L'enveloppe mémoire**, confirmée par G (optimiseur) le 15 novembre sur une portée jouet
(`systemd-run --user --scope -p MemoryMax=100M`, un enfant qui demande 400 Mo). `poste` lance le lourd sous
`systemd-run --scope -p MemoryMax=$POSTE_MEM_MAX` (32 Gio ici). Quand l'analyse de l'arbre entier dépasse le
plafond, le noyau tue l'enfant par SIGKILL (`memory.events` : `oom_kill 1`) ; puis, sous
`DefaultOOMPolicy=stop` (la valeur de `systemd --user` ici), systemd arrête la portée et envoie un SIGTERM au
parent, le confineur, qui dit « signal reçu : 15 ». Le journal systemd ne dit rien.

## Ce qu'il faudrait

Que le confineur dise la cause : après le lot de G, « enveloppe mémoire crevée (N Gio, plafond M Gio, k oom_kill
dans <portée>) » au lieu de « signal reçu : 15 ». Le ticket se ferme avec ce lot. En attendant, une grosse analyse se mesure sur une base (la
sonde de banc), pas par l'analyse en mémoire.
