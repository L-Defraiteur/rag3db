# Le code coloré de Luciform Hub

7 novembre 2026, session tty, à la demande de Lucie par c7. Recherche en lecture seule dans `~/LR_CodeRag/ragforge/packages/luciform-hub` à luciepc. Rien n'est repris : ceci est un relevé.

## Ce qu'il y a

| Fichier | Date | Quoi |
|---|---|---|
| `app/components/ChatWidget.tsx` (1 440 lignes) | 26 févr. 2026 | le chat « Lucie Defraiteur » du site : Markdown et code coloré pendant le flux |
| `app/components/CodeBlock.tsx` (47 lignes) | 18 déc. 2025 | un bloc de code des pages, coloré en TypeScript ou JavaScript seulement |

Les dépendances du `package.json` : `react-markdown` ^10.1, `remark-gfm` ^4.0.1, `react-syntax-highlighter` ^16.1, `highlight.js` ^11.11.1, sous Next 16.0.10 et React 19.2.1.

## Le chat (`ChatWidget.tsx`)

- **Le Markdown.** `ReactMarkdown` avec `remarkGfm` (tables, listes de tâches, liens nus). Chaque élément passe par `components={{…}}` : `code`, `pre`, `p`, `ul`, `ol`, `h1`-`h3`, `strong` et `a`.
- **Les blocs de code.** `Prism as SyntaxHighlighter` de `react-syntax-highlighter`, thème `oneDark`. La langue vient de la classe `language-xxx` du bloc (`/language-(\w+)/`), et vaut `text` sans elle. Les options : `PreTag="div"`, `wrapLongLines`, des marges de 0,5 rem, des coins arrondis, du 0,75 rem, et un défilement horizontal autour (`overflow-x-auto`). Prism colore toutes ses langues, sans en choisir.
- **Le code en ligne.** On le reconnaît à l'absence de classe. Il ne se colore pas : un fond ardoise et une teinte cyan (`bg-slate-700/50 text-cyan-300`), puis `break-all`.
- **Les liens.** `a` s'ouvre dans un nouvel onglet (`target="_blank"`, `rel="noopener noreferrer"`), en cyan. **Il n'y a aucun lien vers un fichier** : un `chemin/fichier.ts` cité reste du texte.
- **Le flux.** Un marqueur `█` porte le curseur. Le dernier mot écrit « glitche » quelques images à 25 ms (`FastGlitchWord` : ses deux dernières lettres tirées au hasard, avec un halo cyan), puis se fige. Le curseur est un bloc cyan.
- **Les appels d'outils.** Une ligne `⚡ <nom>`, puis `⏳` qui pulse tant que l'appel est en cours et `✓` une fois fait. Ni les arguments ni le résultat ne s'affichent.

## Le bloc des pages (`CodeBlock.tsx`)

`highlight.js/lib/core`, avec deux langues enregistrées (typescript et javascript) et le thème `atom-one-dark.css`. Un titre facultatif est précédé d'une pastille de couleur. Le fond est `#282c34`.

## Ce que ça donne pour nous

Lucie veut du code coloré, `code.truc` en lien, et des retours d'outils formatés pour l'humain sans perdre une information :

- **Ce qui se reprend comme idée :** les composants du Markdown surchargés un par un, la langue lue du bloc, et le code en ligne distinct des blocs.
- **Ce qui manque au Hub :** les liens vers les fichiers, et les retours d'outils (ni arguments ni résultat). C'est tout le lot du chat de hooks-essai.
- **Une piste commune à l'IDE et au terminal :** colorer par **tree-sitter**, que codeparsers porte déjà avec ses grammaires (Rust, TypeScript, Python, C/C++, Go, C#, CSS, HTML, Bash). Les deux interfaces auraient les mêmes couleurs, et un `code.truc` cité pourrait se résoudre en lien par l'index du dépôt plutôt que par une expression régulière.
  - Pour le terminal, ce serait `tree-sitter-highlight` (MIT), de quelques Ko par bloc et sans le moteur.
  - Pour l'IDE, le même surlignage côté hôte, ou Prism ou Shiki dans la page s'il faut aller vite.
  - Le choix est à faire avec hooks-essai, puis avec Lucie s'il engage le produit.
