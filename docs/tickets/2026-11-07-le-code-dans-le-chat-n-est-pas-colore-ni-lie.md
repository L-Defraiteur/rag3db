# Le code dans le chat n'est ni coloré ni relié au dépôt

- **État** : ouvert (7 novembre 2026), retour de Lucie sur la pose de 21 h 51 ; pour hooks-essai.
  Fait : la coloration (A, highlight.js, 48abbd62d ; puis B, les grammaires de l'hôte, branche
  coloration-b), le code en ligne, le chemin cité en lien. Reste : le lien vers un symbole.
- **Famille** : produit
- **Gravité** : gêne (un code illisible, des références qui ne mènent nulle part).
- **Atteignable en service** : oui, toute réponse qui montre du code.
- **Touche rag3weaver** : oui (`ui/ide/src/views/Chat.tsx`, le bundle de l'IDE).

## Ce que c'est

Lucie : « il faut qu'il sache mettre des bouts de code ou de la syntax highlight sur demande du
modèle, ou genre `code.truc` pris en compte ». Le Markdown des réponses (marked + DOMPurify) rend
les blocs clôturés en `<pre><code>` sans couleur. Le code en ligne n'est qu'une police, et un
symbole ou un chemin cité (`code.truc`, `src/x.rs:12`) ne mène pas à sa vue. Référence de Lucie :
LR_CodeRag, sur Luciform Hub, avait une coloration syntaxique ; tty connaît ce dossier.

## Ce qu'il faut pour le fermer

- Les blocs clôturés, avec leur langue, sont rendus en code coloré par une bibliothèque de
  coloration dans le bundle. Langues du dépôt : rust, c++, ts, py, bash, json, mermaid…
- Le code en ligne est rendu en code.
- Un chemin (`src/x.rs:12`) ou un symbole connu cité par le modèle devient un lien vers la vue du
  fichier, à sa ligne, ou vers celle du symbole.

Témoins navigateur : un bloc rust coloré ; un chemin cité qui mène au fichier, à la ligne.

## Où on en est (8 novembre 2026)

- **La coloration B** (branche coloration-b) : l'hôte colore par les grammaires tree-sitter de
  codeparsers (`POST /highlight` sous le jeton porteur, `POST /ide/highlight` sous le cookie).
  Les spans sont rendus aux deux décalages, octets et UTF-16. Dans la page, un bloc s'affiche
  d'abord sous highlight.js (`data-hl="repli"`), puis en classes `hl-*` dès que l'hôte répond
  (`data-hl="hote"`). Une langue que les grammaires ne connaissent pas garde le repli.
  Témoins : `l_hote_colore_par_les_grammaires_aux_deux_decalages` et
  `le_code_du_chat_est_colore_par_les_grammaires_de_l_hote`, rouges sur 6a7ed12d2 (route
  inconnue ; bloc resté sous highlight.js), verts sur f1f048186.
- **Reste** : un symbole connu cité par le modèle (`Backend::run`) qui mène à sa définition. Il
  faut une résolution du nom par l'index (outline ou usages) côté page ; c'est un lot à part.
