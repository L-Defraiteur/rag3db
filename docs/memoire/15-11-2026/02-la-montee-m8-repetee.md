# La montée M8 de la mémoire de Lucie, répétée sur une copie

*15 novembre 2026, 4 h, session mémoire (96), sur la demande de l'orchestration. Rien n'a été fait sur
la vraie base. La pose se fait le matin, dans une fenêtre annoncée, quand Lucie est là pour voir ses
gestes.*

## Ce qu'est cette montée, et ce qu'elle n'est pas

**La commande `templates upgrade` n'existe pas encore.** Le ticket M8 reste ouvert : seule sa première
tranche est sur master (`8b3fab422`, un plan à blanc), sans apply ni adoption. Cette montée est donc
**une pose à la main**, que la vraie M8 remplacera.

- **Elle pose le `templates/` livré à la place de la copie de configuration** de Lucie
  (`~/.config/rag3weaver/memoire-longue/`, une copie du 25 octobre).
- **Elle garde ses réglages propres.** Son manifeste ne diffère de celui de master que par `database`
  et `files_root`.
- **Ni la base ni les fichiers ne changent.** Ses entités ne diffèrent de master que par ce qui a été
  ajouté depuis (la transition `recopy`, l'état `recopied`).
- **Elle apporte sept outils** : `archive`, `confirmer_fiche`, `scinder_fiche`, `rejoindre_fiches`,
  `select_apports`, `select_sources` et `whois`.

**Pour Lucie : son manifeste n'a pas de `models.llm`.** La montée ne l'ajoute pas, parce que c'est son
choix de modèle. Les trois gestes n'en ont pas besoin. `archive`, lui, dira « sans modèle déclaré »
tant qu'elle n'en a pas posé un.

## La répétition (luciepc, `~/.cache/rag3weaver-build/m8-repetition/`)

- **La copie.** L'instantané complet le plus récent, `avant-chantiers-14-nov-2` (18 h 20, 116 fiches),
  sommes vérifiées. La copie de configuration a ses chemins réécrits vers la copie de la base avant
  tout appel.
- **Le binaire.** Bâti sur master `26a091474`, lib 373b5477b.
- **Le pilote** : `monter-m8.sh <config> <templates livrés> <binaire> <journal>`.
  - Il pose le livré dans un dossier à côté et garde les réglages.
  - Il **charge le manifeste monté avant de basculer** : s'il ne se charge pas, rien n'est basculé.
  - Il bascule (l'ancienne configuration est renommée `.avant-m8-<date>`) et écrit `RETOUR.sh`, qui
    la remet telle quelle.

| | avant | après la montée | après les gestes |
|---|---|---|---|
| fiches en base | 116 | 116 | 118 |
| fichiers de fiche | 116 | 116 | 118 |
| sources | 63 | 63 | 63 |
| outils servis | 20 | 27 | 27 |

**Les trois gestes, sur la copie :**

- **`confirmer_fiche`** sur « Classeur local du rappel » : proposed → current, révision 2.
- **`rejoindre_fiches`** de « Hooks de rag3weaver » vers « hooks Claude Code de rag3weaver » : 4 apports
  recopiés, la première retirée, la seconde garde son état et porte l'alias.
- **`scinder_fiche`** de « L'équipier » (15 sections) en « L'équipier, la conception » et « L'équipier,
  les essais » : 15 apports recopiés, l'ancienne retirée, les deux neuves proposées.

## La pose du matin, dans l'ordre

1. **G pose sur l'hôte du portable un binaire au moins égal à `1f5d007b8`**, avec son verdict. Les
   outils neufs citent des graphes intégrés au binaire (`@builtin:…gestes_un.mmd`), et le binaire
   d'aujourd'hui, `fac67a208`, est d'avant eux : sans ce binaire, le manifeste monté ne se charge pas
   et le pilote ne bascule rien.
2. **G ferme `memoire-longue` avec une raison** : la fermeture qui tient, posée sur l'hôte du portable
   depuis 1 h 15. Je prends l'instantané complet.
3. **La montée**, avec `templates/` pris au commit du binaire posé :

       ~/.cache/rag3weaver-build/m8/monter-m8.sh ~/.config/rag3weaver/memoire-longue \
         <checkout de master>/extension/rag3weaver/templates <binaire posé> \
         ~/.cache/rag3weaver-build/m8/montee.log

4. **G rouvre.** Je vérifie que l'hôte sert les 27 outils, et je compte les fiches en base et en
   fichiers, à comparer à l'instantané.
5. **Lucie fait ses gestes** depuis la page de survol (`01-…` et `14-11-2026/02-…`).

**Pour revenir en arrière** : G ferme, `~/.cache/rag3weaver-build/m8/RETOUR.sh`, G rouvre. La
configuration d'avant revient telle quelle. La base n'a pas été touchée par la montée, et ce que les
gestes ont écrit se défait par l'instantané.
