# Trois thèmes de plus : cyberpunk, cyborg, alien

13 novembre 2026, session tty, sur le mot de Lucie (22 h) : « faut un thème cyberpunk et un thème cyborg aussi et un thème alien ». Chacun est un fichier de `extension/rag3weaver/templates/themes/`, de la forme exacte de « daemon » et de « forêt ». On les choisit par `--theme cyberpunk|cyborg|alien` ou par `RAG3WEAVER_TTY_THEME`. Lucie tranche à l'écran : tout se change dans le fichier, sans toucher au code.

## Les trois univers

| Thème | Fond et texte | Cercle (réfléchit) | Transmutation (lit) | Glitch (écrit) |
|---|---|---|---|---|
| **cyberpunk** | magenta néon `#ff2bd6` sur nuit violette `#0d0221`, halo cyan `#00f0ff` → violet `#6a1fb0` | blocs `▚ ▞ ◢ ◣ ◤ ◥ ▣` autour de `◉`, « JACKING IN… » | une trame `░ ▒ ▓ █` qui monte et descend, « SCANNING THE GRID… » | `0 1 # % & $ @ ¥ ¤` : « OVERCLOCKING », « HACKING THE MAINFRAME », « WRITING TO THE NET » |
| **cyborg** | chrome `#d7dde5` sur gris métal `#1c1f24`, halo ambre d'alerte `#ffb000` → acier `#5a6270` | `⊕ ⊗ ⊙ ⊚ ⊛ ⊜ ⊝` autour de `⏣`, « DIAGNOSTIC EN COURS… » | une jauge `▁ ▂ ▃ ▄ ▅ ▆ ▇`, « LECTURE DES CAPTEURS… » | `⌁ ⌇ ⎓ ⏚ ⎍ ⌬ ⍜ ⏛ ⎔` : « ALERTE : ÉCRITURE », « SERVOMOTEURS ENGAGÉS », « SOUDURE DU CODE » |
| **alien** | vert acide `#39ff14` sur noir-vert `#020d06`, halo bioluminescent `#b6ff3b` → `#0b5e3c` | une écriture inconnue `⍜ ⏁ ⏂ ⎎ ⍙ ⏃ ⎅` autour de `◬`, « SIGNAL REÇU… » | une pulsation `· ∘ ○ ◌ ◍ ◎ ●`, « ÉCHANTILLONNAGE… » | `⟟ ⟒ ⋔ ⍀ ⌰ ⏁ ⏚ ⌖ ⟊` : « MUTATION DU CODE », « LA RUCHE ÉCRIT », « PONTE DANS LE NOYAU » |

Le code de chaque thème a sa palette des 15 sortes, dans le même univers. Tous les glyphes font une colonne (aucun du plan astral), et chaque thème a son jeu ASCII.

## La règle de contraste, la même pour les cinq thèmes

Le témoin `theme::tests::every_theme_keeps_a_minimal_contrast_against_its_background` mesure, pour chaque thème, le contraste WCAG 2.x de chaque couleur contre le fond.
- **4,5 au moins** (le niveau AA du texte courant) pour le texte, et pour le code qui se lit : mots-clés, types, fonctions, chaînes, nombres, variables, propriétés, opérateurs, attributs, labels, modules, `other` et `en_ligne`.
- **3 au moins** pour ce qui accompagne : la sourdine, les commentaires, la ponctuation et le cadre des blocs.
- Les couleurs du halo ne portent que des glyphes décoratifs : elles n'entrent pas dans la règle.

**Elle a trouvé un défaut dans « daemon ».** Ses attributs, labels et modules (`#b45a5a`) n'avaient que 4,42 de contraste sur `#050505`. La valeur passe à `#b85d5d` (4,62), la nuance la plus proche qui tienne la règle. Aucun champ n'est renommé ni retiré, et l'IDE, qui lit le même fichier, prend la nouvelle valeur tel quel.

## Les témoins (le rouge d'abord)

Écrits avant les fichiers, ils étaient rouges :
- `the_cyberpunk_cyborg_and_alien_themes_are_there` : « pas de thème « cyberpunk » ; thèmes : daemon, foret » ;
- la règle de contraste : `daemon : code.attribute #b45a5a sur #050505 fait 4.42, il faut 4.5`.

Ils sont verts depuis, de même que les témoins de forêt, qui couvrent désormais les cinq thèmes : l'embarqué octet pour octet, la forme et la largeur des glyphes, l'ASCII pur, et les couleurs du Cercle jusqu'aux cellules. Le crate passe 35/35 avec la vue et 7/7 sans.
