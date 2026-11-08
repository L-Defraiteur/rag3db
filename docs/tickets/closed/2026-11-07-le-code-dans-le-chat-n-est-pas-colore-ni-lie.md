# Le code dans le chat n'est ni coloré ni relié au dépôt

- **État** : corrigé (8 novembre 2026) : la coloration A (384f6fac5) et B (5ec68e9a5), le code en
  ligne et le chemin cité en lien (384f6fac5), le symbole cité en lien vers sa définition
  (36aced578, session archi). Retour de Lucie sur la pose de 21 h 51.
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

## Le symbole cité (8 novembre 2026, archi, 36aced578)

`POST /ide/symboles {backend, names}` résout en lot par `usages`, en appel discret
(`Backend::call_discret` : son run va sur un bus à part, ni journal ni barre), sans ouvrir de
dépôt ni attendre un catalogue tenu, sans lien sur un index partiel. Dans la page, un code en
ligne en forme d'identifiant devient un lien vers le fichier à sa ligne (une définition) ou vers
la vue du symbole (plusieurs) ; les mots du langage ne sont pas demandés, le cache vit cinq
minutes et se vide à un événement `files`. Témoins : `un_symbole_cite_se_resout_par_l_index_sans_ouvrir_de_depot`,
`un_catalogue_tenu_rend_tout_de_suite_sans_attendre`, `un_rendu_n_ecrit_aucun_run_au_journal`,
`un_symbole_cite_par_le_modele_mene_a_sa_definition`.
