# Un thème commun au terminal et à l'IDE : « daemon »

7 novembre 2026, session tty. Lucie veut le « Daemon Code Summoning » sur l'état « le modèle réfléchit » de l'IDE, en plus du terminal. Cette page propose des **tokens communs**, pour que les deux aient le même thème. C'est une proposition à arrêter avec everything declarative ; rien n'est codé.

## La source

- La page d'origine, telle quelle : `05-daemon-code-summoning.html`, dans ce dossier. Elle sort du commit `a25f3ecf9` de ragforge (10 déc. 2025), fichier `docs/project/2025-10-dec-18h10/Agent_terminal_beautification.md`, dont on a retiré les deux lignes d'en-tête.
- Ce qu'on en retient : page 01 ; ce que Lucie en a dit : page 02, § « Le design : la page HTML d'abord, puis des thèmes ».

## Les tokens proposés

Un seul fichier JSON par thème, lu par les deux :
- par le terminal (le crate `extension/rag3weaver/tty`, par `include_str!`) ;
- par l'IDE (embarqué avec ses gabarits, ou importé par le bundle).

Son chemin est à convenir, par exemple `extension/rag3weaver/templates/themes/daemon.json`.

```json
{
  "nom": "daemon",
  "couleurs": {
    "fond": "#050505",
    "texte": "#ff3333",
    "halo": ["#ff0000", "#8b0000"],
    "sourdine": "#666666",
    "ansi256": { "texte": 196, "halo": 88, "sourdine": 242 },
    "ansi16": { "texte": "rouge vif", "halo": "rouge", "sourdine": "gris" }
  },
  "police": "'Courier New', Courier, monospace",
  "animations": {
    "reflechit": {
      "forme": "cercle",
      "ms": 150,
      "glyphes": ["᚛", "ᚨ", "ᛒ", "ᛟ", "᚜", "⸸", "‡"],
      "coeur": "⛧",
      "gabarit": "⟪ {gauche} {coeur} {droite} ⟫ INVOCATION DU CODE..."
    },
    "lit": {
      "forme": "transmutation",
      "ms": 200,
      "glyphes": ["⍟", "🜂", "☿", "☉", "♄", "🜄", "∮"],
      "gabarit": "⁅ ⸸ ⁆—[ {s} ]—⁅ ⸸ ⁆ ANALYSE EN COURS..."
    },
    "ecrit": {
      "forme": "glitch",
      "ms": 100,
      "glyphes": ["∇", "∫", "∃", "∀", "∴", "∵", "⊕", "⊗", "⌇"],
      "coeur": "⛧",
      "gabarits": [
        "[{a} {coeur} {b}] ...COMPILING CURSE...",
        "❬{a}{b} {coeur} {c}❭ ...SUMMONING COMPILER...",
        " {a} ⸢{coeur}⸣ {b}  ...PARSING SOULS..."
      ]
    }
  },
  "replis": { "🜂": "△", "🜄": "▽" },
  "ascii": { "coeur": "*", "glyphes": ["|", "/", "-", "\\"] }
}
```

## Les règles communes

| Règle | Ce qu'elle dit |
|---|---|
| **Quand chaque animation** | `reflechit` : de l'appel au modèle au premier token, et pendant le raisonnement. `lit` : un outil qui lit (`policy.read_files` sans écriture ni exécution). `ecrit` : un outil qui écrit ou exécute (`write_files` ou `run_commands`). Le choix se fait sur la `policy` que `describe` porte depuis `cfc5cc8e8`, jamais sur une liste de noms. |
| **La cadence est bornée** | Jamais plus de 10 images/s pour une animation (100 ms au plus vite, la cadence du Glitch). Le terminal dessine en plus l'écran au plus 30 fois par seconde. |
| **Le hasard est reproductible** | Le Glitch tire ses glyphes d'un générateur à graine (un xorshift), fixée dans les tests ; c'est ainsi qu'on en prend un instantané. |
| **Le halo** | Le navigateur le dessine tel quel, par `text-shadow: 0 0 5px #ff0000, 0 0 10px #8b0000`. Le terminal l'imite : `⛧` en gras dans la couleur `texte`, les runes en dégradé de `halo[1]` vers `texte` selon leur place dans le cercle. |
| **Les dégradés** | Terminal : couleur vraie, puis 256 couleurs, puis 16, puis `NO_COLOR` (gras et normal). IDE : `prefers-reduced-motion` fige l'animation sur sa première image. |
| **Les glyphes** | Le terminal mesure sa largeur au démarrage et prend les `replis` quand un glyphe ne fait pas une colonne, ou le jeu `ascii` sur demande. L'IDE peut se fier à sa police, mais garde les `replis` pour une police qui n'a pas le plan astral (`🜂 🜄`). |
| **Les thèmes à venir** | fées, forêt, bureau, papillons, Mordor, Dracula au sens vampire (Lucie). Chacun est un autre fichier de même forme : mêmes clés, autres glyphes, autres couleurs, autres libellés. |
