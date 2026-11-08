# Le code dans le chat n'est ni coloré ni relié au dépôt

- **État** : ouvert (7 novembre 2026), retour de Lucie sur la pose de 21 h 51 ; pour hooks-essai,
  après le lot appel-a-delai.
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
