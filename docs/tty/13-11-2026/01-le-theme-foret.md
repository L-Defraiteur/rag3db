# Le thème « forêt »

13 novembre 2026, session tty. C'est le deuxième thème du terminal, après « daemon ». Il vient de la liste de Lucie : fées, forêt, bureau, papillons, Mordor, Dracula.

## Ce que fait le lot

- **Le fichier** : `extension/rag3weaver/templates/themes/foret.json`. Il a la forme exacte de `daemon.json` : couleurs en hex, trois animations en données, `replis`, `ascii`, et la palette `code` des 15 sortes de spans plus `en_ligne` et `cadre`. Aucun champ n'est renommé ni retiré.
- **Le choix du thème** : `rag3weaver-tty --theme foret` ou `RAG3WEAVER_TTY_THEME=foret`, et « daemon » par défaut. Un nom inconnu est refusé avant l'ouverture de l'écran, et le message liste les thèmes.
- **Les thèmes embarqués** : `tty/src/theme.rs`, `THEMES`. Chaque fichier de `templates/themes/` y est embarqué par `include_str!`.

## La forêt

| Animation | Quand | Glyphes | Libellé |
|---|---|---|---|
| Cercle, 150 ms | l'agent réfléchit | `❦ ❧ ☘ ✿ ❀ ♣ ⚘` autour de `⸙` | « LA FORÊT ÉCOUTE… » |
| Transmutation, 200 ms | un outil lit | phases de lune `☽ ◐ ◓ ◑ ◒ ☾ ●` | « LES RACINES CHERCHENT… » |
| Glitch, 100 ms | un outil écrit ou exécute | `⁂ ∴ ⁖ ⁘ ⁙ ⸪ ⸫ ⸬ ⸭` | « LA SÈVE MONTE… », « LES FEUILLES ÉCRIVENT… », « L'ÉCORCE SE GRAVE… » |

Les couleurs : mousse `#8fd16a` sur `#07130b`, un halo du vert tendre `#c5f07a` au vert profond `#2f5d2a`, et une sourdine `#5f6e58`. Le code est en verts, ocres et écorce. Tous les glyphes font une colonne (BMP, aucun du plan astral), donc `replis` est vide. En ASCII, le cœur devient `&`, et les lettres accentuées des libellés passent par `cadre`.

## Les témoins (le rouge d'abord)

Écrits avant le fichier, ils étaient rouges : `the_forest_theme_is_there_and_named_foret` échouait sur son assertion, le thème n'existait pas. Ils sont verts depuis :

| Témoin | Ce qu'il vérifie |
|---|---|
| `theme::tests::every_templates_theme_file_is_embedded_byte_for_byte` | chaque fichier du dossier commun est embarqué, octet pour octet, et aucun thème embarqué ne manque au dossier |
| `theme::tests::every_theme_has_the_daemon_shape_and_one_column_glyphs` | la forme de chaque thème |
| `anim::tests_de_tous_les_themes::ascii_mode_is_pure_ascii_for_every_theme` | l'ASCII pur, pour chaque thème |
| `vue::tests::every_theme_colours_its_circle_cells` | le cœur dans `texte` en gras et le glyphe du cercle dans le halo, pour chaque thème |

Le crate passe 33/33 avec la vue et 7/7 sans.

## Pour l'IDE

L'IDE ne sert aujourd'hui que `/ide/theme/daemon.json`. Archi a dit qu'il ferait la route `/ide/theme/<nom>.json` et un menu quand ce premier thème suivant arriverait : je le préviens au push.

## Pour voir le thème

```bash
cd extension/rag3weaver/tty && cargo run --release -- --theme foret --depot <dépôt attaché> --tache essai-foret --demande "Lis src/lib.rs, puis montre un exemple dans un bloc de code."
```

`--demande` lance un vrai équipier, avec un vrai modèle.
