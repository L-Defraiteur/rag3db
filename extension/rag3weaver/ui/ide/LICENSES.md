# Les paquets du bundle de l'IDE et leurs licences

Lues dans `node_modules/<paquet>/package.json` (champ `license`) le 25 octobre 2026, versions
épinglées dans `package.json` et `package-lock.json`.

| paquet | version | licence | rôle |
|---|---|---|---|
| react, react-dom | 19.2.0 | MIT | le rendu |
| lucide-react | 1.61.0 | ISC | les icônes |
| radix-ui | 1.4.3 | MIT | les primitives des composants shadcn/ui copiés |
| class-variance-authority | 0.7.1 | Apache-2.0 | les variantes des composants |
| clsx | 2.1.1 | MIT | les classes conditionnelles |
| tailwind-merge | 3.3.1 | MIT | la fusion des classes |
| marked | 18.1.1 | MIT | le Markdown des réponses du chat |
| dompurify | 3.4.17 | (MPL-2.0 OR Apache-2.0) | l'assainissement du HTML rendu (le texte vient d'un modèle) |
| esbuild | 0.28.2 | MIT | le bâti du script (outil, pas servi) |
| tailwindcss, @tailwindcss/cli | 4.3.3 | MIT | le bâti de la feuille (outil, pas servi) |
| typescript | 5.9.3 | Apache-2.0 | la vérification des types (outil, pas servi) |

Les composants sous `components/ui/` sont copiés depuis shadcn/ui (MIT) par sa CLI
(`shadcn@4.22.1 add …`), comme la bibliothèque le prévoit : ils appartiennent au dépôt.
