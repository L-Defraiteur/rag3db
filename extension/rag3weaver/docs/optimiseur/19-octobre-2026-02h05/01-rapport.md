# Rapport de G, nuit du 18 au 19 octobre 2026

Session G (optimiseur, `rag3db-90`). Consignes de l'orchestration (c7), du proto, de codeparsers, de
l'arbre principal et du banc. Lucie n'a écrit aucun message cette nuit.

## Ce qui est sur master

| commit | lot |
|---|---|
| `bacf411e1` | **Le témoin de la page de l'IDE dans un vrai navigateur** (`src/daemon_host/navigateur.rs`) : Chromium sans écran par le protocole DevTools sur un tube, un vrai hôte qui sert le gabarit `code` ; l'accueil, l'arbre, un fichier, un symbole, la recherche, le graphe et l'aperçu d'un composant, zéro erreur à la console, puis une édition enregistrée et relue sur le disque. Trois défauts de la page corrigés au passage (ci-dessous). |
| `9c045f684` | **La CI de master ne s'annule plus** : CI Workflow tuait son run en cours à chaque push ; rag3weaver le faisait déjà bien (ses « cancelled » sans job sont des runs en attente remplacés par le plus récent). |
| `a0a403e28` | **Le témoin contre un vrai hôte, en lecture seule** (`l_ide_d_un_vrai_hote_se_rend_en_lecture_seule`, ignoré par défaut, piloté par `RAG3WEAVER_TEMOIN_HOTE` et `RAG3WEAVER_TEMOIN_MANIFESTE`) ; et les fd 3 et 4 du navigateur passés d'abord au-dessus de 10 (un tube déjà en fd 3 restait en CLOEXEC). |
| `bf046e540` | **L'hôte rend la mémoire que glibc garde** (`src/allocateur.rs`) : `malloc_trim(0)` en fin d'index, à la fermeture d'un backend et au repos ; aucun `MALLOC_ARENA_MAX` (mesuré trop cher en débit). Ticket fermé, jugé par le banc ; la phase des vecteurs part dans un ticket neuf. |

Les trois défauts de la page que le témoin navigateur a trouvés, tous silencieux pour les témoins
qui ne lisaient que les appels :
1. `/favicon.ico` refusé (403) hors de la porte de l'IDE : deux erreurs à la console par vue. Une
   icône vide.
2. Le symbole, l'impact et les appelés lisaient l'enveloppe `{status, result}` des lectures du
   catalogue et montraient zéro. `appeler()` la défait ; un refus devient l'erreur affichée, la
   ligne d'état se montre en tête de la vue.
3. À la septième page, la vue restait sur « Chargement… » : les pages quittées, gardées en cache
   d'historique, tenaient leur flux SSE, et le navigateur ne donne que six connexions par hôte. La
   page ferme son flux au `pagehide`.

## Les hôtes

- **Ce poste (Lucie)** : binaire `f95dd559` sur `bf046e540`, lib `a333c34f2` (le ROLLBACK sous
  tampon plein et les deux correctifs du cœur), installé à 2 h 01. Le filet
  `RAG3WEAVER_LINK_COPY_THRESHOLD` est retiré de son env (le lot des relations, `66bf6a059`, est sur
  master). Jeton inchangé, la mémoire longue répond par ses vecteurs, et **le témoin en lecture
  seule contre son hôte est vert** (les six vues en 5,8 s).
- **Ses copies de gabarits rafraîchies.** Son backend `code-rag3db` prenait ses outils dans une copie
  du 11 octobre : son `list_files` refusait `format`, et l'arbre de l'IDE lui aurait montré une
  erreur. Un relevé compare chaque fichier de ses copies aux gabarits livrés : ce qui vaut une version
  passée du livré est rafraîchi (gardé en `<fichier>.avant-19-oct`) ; ce qui ne vaut aucune version
  livrée est à elle et reste. `code-rag3db/templates` : 18 rafraîchis ; `memoire-longue` : 19
  rafraîchis, `backends/memory/backend.json` gardé (modifié par elle). Aucune base touchée.
- **luciepc** : basculé par codeparsers, vérifié par `/proc` ; sur `047655729` puis son correctif de
  la borne d'ambiguïté, sans le filet du COPY.

## En cours, ou à reprendre

- **Le lot des gabarits désignés par leur nom** (demande de c7) : un manifeste installé désigne un
  gabarit livré par son nom, qui suit chaque rebâti, et ne copie que ce que l'utilisatrice a
  modifié ; l'hôte dit à l'ouverture qu'une copie locale diffère du gabarit livré du même nom, et la
  page le montre. Repérage fait : le binaire n'embarque aujourd'hui que 13 graphes d'outils
  (`graph_tool.rs:1757-1796`), aucun manifeste ne les désigne par un nom, et chaque chemin se lit par
  `directory.join(…)` dans `PreparedBackend::load`.
- **La mémoire d'un index avec ses vecteurs** (ticket neuf) : passes A et B avec embarquement, quand
  les services de luciepc sont libres. Les binaires A et B2, le corpus (`rag3db-corpus-c6bb`) et le
  pilote sont gardés à luciepc pour cela.
- **CodeMirror pour l'édition** (B1), après les lots du proto ; le lot 4 (React à chaud) est sur
  master.
- **L'alpha.3 du paquet npm** : les correctifs moteur et le lot des relations sont sur master ; reste
  à rebâtir les archives, sans publier (OTP avec Lucie).
