# L'oracle cherche la cible au lieu du nom réellement mentionné

- **État** : en cours (13 novembre 2026) — l'oracle interroge le nom réellement mentionné (branche rag3db
  `codeparsers/oracle-mention`, 985d91d88) ; les 415 liaisons rejugées ; restent les témoins alias et
  instance, et la ligne agrégée des sites (complément ci-dessous)
- **Gravité** : preuve incomplète (un faux peut devenir muet sans interrogation du LSP)
- **Atteignable en service** : non (oracle de mesure seulement)
- **Touche rag3weaver** : oui (outillage de validation des arêtes)
- **Famille** : produit

- **Pour** : session codeparsers
- **Signalé par** : session codeparsers ; reprise et preuves dans [le bilan du 11 novembre](../11-11-2026/04-bilan-navigation-et-provenance.md)

## Preuve et cause

La sonde A/B fournit `b.name` comme champ nom de l'arête. Dans
`scripts/oracle_lsp.py`, la boucle cherche `a['nom']` sur la ligne source,
puis interroge le LSP seulement si ce texte y apparaît. Le verdict commence
à muet. Une cible zone de fichier, ou une classe exportée dont le nom diffère
de l'alias importé, peut donc être rendue muette sans question LSP.

Vérification à luciepc : les 415 lignes marquées liaison de
`~/.cache/codeparsers-notes/require-mesure-2/verdicts-ajoutees.tsv`
ont été comparées à leurs lignes sources dans `oracle/arbre`, avec la même
regex que l'oracle : 415 tokens absents, zéro source absente, zéro token présent.
Le rapport ancien qui les décrit comme un silence du serveur ne démontre donc
pas ce silence. Il démontre une absence de jugement.

L'instance FS corrigée vers sa zone et un alias Alias vers une classe Outil
rencontrent la même limite. Une baisse des faux accompagnée de nouveaux muets
ne prouve pas un gain de précision.

## Invariant requis pour le fermer

Conserver séparément l'identité de la mention (texte et position source) et la
cible proposée (fichier et plage). Interroger la mention réelle et juger la cible
par les positions rendues, sans substituer son nom à celui de la mention.
Une question non envoyée doit se distinguer d'un LSP interrogé sans réponse.
Poser des témoins alias et zone, puis rejouer les mesures concernées avec les
entrées corrigées, en conservant les anciennes preuves et les seuils.
Cette correction d'outillage est distincte du lot de navigation ; aucune
modification de l'oracle n'est autorisée implicitement par ce ticket.

## Complément : la ligne ne désigne pas chaque occurrence

L'examen du producteur d96ed3264/a730a9b montre une seconde perte :
IdentifierReference porte le nom source et une colonne optionnelle ; UsageSite
n'en garde que la ligne et la nature. usage_properties conserve ensuite le
minimum des lignes du groupe. Lire Scope.key et ses bornes améliore
l'attribution au propriétaire sans restituer les occurrences supprimées.
Sur huit usages des index nodejs/sync, les dépendances existent toujours avec
une autre ligne représentative : les retraits de tuples ne prouvent pas une
perte de rappel. Le complément de 23 questions a été joué, avec ses réponses
brutes conservées ; il ne ferme pas ce ticket général.
Voir [le bilan et la chaîne de provenance](../11-11-2026/04-bilan-navigation-et-provenance.md).

## Action bornée proposée et portage

Porteur : codeparsers. Diagnostic terminé dans le bilan db0dbfcbb ; le défaut
et la suite de validation restent ouverts. Prochaine action proposée à
l'orchestration : capture de provenance avant perte sur les mêmes 23 questions,
avec versions/options figées, clés de scopes, tokens et positions vérifiés,
puis réutilisation des réponses LSP existantes. Ne pas attribuer les arêtes
à partir du résultat LSP lui-même. Cette proposition ne déclenche aucune
nouvelle campagne, schéma ou refonte générale sans cadrage supplémentaire.
Reste : valider le périmètre, réaliser la capture, rendre explicites les cas
non attribuables, et satisfaire les critères généraux de fermeture ci-dessus.

## Le 13 novembre : le nom mentionné interrogé, les 415 liaisons rejugées

**La correction** (`scripts/oracle_lsp.py`, `mentions_a_interroger`) : l'oracle interroge le nom de la
cible s'il est sur la ligne ; sinon un nom qu'une ligne d'import lie **dans la cible** (une zone de
liaison) ; sinon un nom qu'un import du fichier de la mention lie **depuis le fichier de la cible** (un
alias). Quand rien sur la ligne ne nomme la cible, aucune question ne part et le verdict est **« non
interrogé »**, distinct de « muet » (interrogé, sans réponse). Le TSV gagne une colonne « nom interrogé ».
Les catégories « liaison d'import » et « réexportation » se jugent sur ce nom.

**Rouge d'abord** : trois des 415 arêtes `liaison`, sous l'oracle de master, « muet » ×3 en 0 s (aucune
question) ; sous l'oracle corrigé, 2 « liaison d'import » (`path`, `rmSync` vers `@types/node`) et
1 « muet » interrogé (une chaîne, ci-dessous). Fichiers : `~/.cache/codeparsers-notes/oracle-mention/`
à luciepc (`verdicts-avant.tsv`, `verdicts-apres.tsv`).

**Les 415** (`liaisons-415.txt`, `verdicts-415.tsv`, même arbre a72acfeef, typescript-language-server sur
TypeScript 5.9, juge strict, 2 590 s) :

| verdict | nombre | ce que c'est |
|---|---|---|
| liaison d'import | 376 | le serveur rend le module externe (`@types/node`…) ; notre cible est la ligne qui le lie |
| juste | 32 | le serveur rend la ligne de liaison elle-même (un module sans types : `chromium`, `uuidv4`) |
| muet | 4 | interrogé sans réponse : **les quatre nomment le mot dans une chaîne** (`'path'`, `'.result-path'`, `'mtga-…'`) — de vrais faux de l'index (la lecture du texte des zones prend des mots dans les chaînes) |
| faux | 1 | `tools/nodejs_api/install.js:103`, `path.toLowerCase()` : `path` y est un paramètre local qui masque l'import |
| non interrogé | 2 | rien sur la ligne ne nomme la cible (`process.env` à `install.js:69`, `join(…)` à `smoke_warnings.mjs:11`) |

**408 sur 415 (98 %) sont saines** ; 7 sont des faux de l'index, dont la famille principale (4) est la
lecture des chaînes.

**Reste pour fermer** : les témoins alias (`const Alias = require("./outil")` vers `class Outil`) et
instance (`module.exports = new FS()`) sous l'oracle corrigé ; la ligne agrégée des sites (le complément
plus haut) ; rejouer l'échantillon de référence TypeScript (les 298 mentions) avec l'oracle corrigé.
