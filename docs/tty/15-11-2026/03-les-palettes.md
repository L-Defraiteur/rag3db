# tty-12 : des palettes nommées, et le code qui en découle

Session tty, 15 novembre 2026, au mot de Lucie : « avoir vraiment des palettes à chaque fois bien strictement définies, plusieurs couleurs nommées, et ensuite changer de thème les change juste, ainsi que la coloration syntaxique : on met un offset dessus en originant la couleur la plus proche de celle qu'on veut atteindre ». Elle s'inspirait de `ltc-color-palettes` et de `kirklin/palette`, et a choisi deux points : OKLCH, et la conversion des cinq thèmes au plus près de leurs couleurs. Le format a été vu avec archi avant le code, en cinq points.

## Le format

- **Un thème est une palette** : `palette` porte huit couleurs nommées, chacune `{"oklch": [L, C, h], "role": …}`. Pour daemon : `nuit` (fond), `cendre` (neutre), `sang` (accent), `ecarlate` (halo), `caillot` (halo2), `braise`, `soufre` et `ocre` (teintes). La forme OKLCH est canonique. Une couleur donnée en hex (`{"hex": …}`) se convertit une fois, et le script réécrit sa forme OKLCH.
- **Un mode** : `mode` vaut `sombre` ou `clair`. Les cinq thèmes sont sombres.
- **Les rôles communs** vivent dans `extension/rag3weaver/templates/theme-roles.json`, hors de `templates/themes/`, que l'hôte lit en entier comme des thèmes (le premier point d'archi). Pour chaque mode, ils disent :
  - quelle couleur prend chaque rôle de `couleurs` : le fond, le texte, la sourdine et les deux couleurs du halo ;
  - comment se dérive chaque sorte de `code`, soit à partir d'un rôle (`"depuis": "accent"`), soit de la couleur chromatique la plus proche d'une teinte visée (`"teinte": 100`, en degrés OKLCH), avec une clarté et une part de chroma.
- **Une surcharge par thème** (`roles` dans son JSON), prioritaire sur les rôles communs (le quatrième point d'archi).
- **Les champs dérivés** (`couleurs.*`, `code.*`) restent dans chaque JSON, que le terminal et l'IDE lisent tels quels : `lib/theme.ts` n'a rien à changer. La clé `derive` dit qu'ils ne s'éditent pas à la main.

## La dérivation, un seul rédacteur

`extension/rag3weaver/scripts/themes_palettes.py` est seul à écrire les champs dérivés (le deuxième point d'archi). Une sorte de code part d'une couleur de la palette, puis s'en décale :
- sa teinte va vers la teinte visée de 35 % de l'écart, au plus 20° : la couleur reste celle du thème ;
- sa chroma est multipliée par la part donnée, et sa clarté prend celle du mode.

Deux règles s'appliquent ensuite :
- **La gamme.** Une couleur hors de sRGB perd de la chroma, à teinte et clarté constantes, jusqu'à y rentrer. Elle n'est jamais coupée canal par canal (le troisième point d'archi).
- **Le contraste**, en WCAG 2 et en luminance relative sRGB (le cinquième point). Il vaut au moins 4,5 contre le fond, et 3 pour la sourdine, les commentaires, la ponctuation et le cadre : la règle même du test Rust. La clarté s'éloigne du fond d'un centième à la fois jusqu'au seuil. Si aucune ne l'atteint, le script s'arrête et le dit, sans jamais rendre une couleur fausse.

Ses trois usages :
- `scripts/themes_palettes.py` réécrit les thèmes ;
- `--check` rougit si un thème diffère de ce que dérive sa palette : c'est la garde contre une retouche à la main ;
- `--essais` joue ses propres essais : OKLCH aller-retour sur sept couleurs, la gamme, le contraste dans les deux modes, un seuil inatteignable qui refuse, l'écart de teinte qui passe par 0°, la couleur la plus proche, une nuance, et un thème clair entier tenu à ses seuils.

## La conversion, au plus près

Les rôles communs ne sont pas réglés à la main. Pour chaque sorte de code, une recherche a choisi, parmi les rôles et les teintes visées de 10° en 10°, les clartés et les parts de chroma, la règle commune la plus proche des couleurs d'avant sur les cinq thèmes, en distance OKLab. Là où la règle commune s'écartait encore de plus de 0,08 pour un thème, une surcharge de ce thème a été cherchée de la même façon : elle part toujours d'une couleur de sa palette. Le fond, le texte, la sourdine et le halo sont inchangés dans les cinq thèmes.

| Thème | Couleurs nommées | Surcharges de code | Écart moyen du code (OKLab) | Écart le plus grand | `couleurs` changées |
|---|---|---|---|---|---|
| daemon | 8 | 8 | 0,022 | 0,040 (property) | aucune |
| foret | 8 | 1 | 0,029 | 0,076 (en_ligne) | aucune |
| cyberpunk | 8 | 1 | 0,017 | 0,063 (operator) | aucune |
| cyborg | 8 | 4 | 0,013 | 0,043 (punctuation) | aucune |
| alien | 8 | 5 | 0,025 | 0,052 (other) | aucune |

En OKLab, un écart de 0,02 se voit à peine ; 0,08 est l'écart le plus grand gardé sans surcharge. Un premier essai, avec des teintes visées choisies à la main, attirait tous les thèmes vers le chaud : le `type` du cyberpunk passait du cyan à l'orange. La recherche a remplacé ce réglage à l'œil.

## Les témoins

Dans le crate tty, chacun rouge avant le code :
- chaque thème porte sa palette, son mode, la mention de ses champs dérivés, et au moins six couleurs nommées, chacune avec sa forme OKLCH et son rôle ;
- le script trouve chaque thème à jour (`--check`) et passe ses essais (`--essais`).

Une garde de compatibilité, verte dès le départ : un thème ancien, sans palette ni mode, se lit encore. Le test de contraste existant tient pour les cinq thèmes dérivés.

Le crate passe de 79 à 82 tests avec la vue, et en garde 14 sans elle.

## Faire un thème neuf

Écrire `nom`, `mode`, et une `palette` de couleurs nommées en hex, avec leurs rôles (fond, neutre, accent, halo, halo2, teintes), puis les animations et les replis comme un thème existant. Lancer `scripts/themes_palettes.py` : il convertit la palette, dérive les couleurs et le code, et refuse si un contraste n'est pas tenable. Enfin, ajouter le thème à la liste embarquée du terminal (`tty/src/theme.rs`).
