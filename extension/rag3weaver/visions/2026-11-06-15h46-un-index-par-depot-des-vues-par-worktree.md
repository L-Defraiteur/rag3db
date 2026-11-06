# Un index par dépôt, des vues par worktree

Lucie, 6 novembre 2026 : « il y a tellement de branches et de PR que faire des
clones à chaque fois de la mémoire me paraît abusé ; il faudrait, dans
l'immédiat, un système plus intelligent qui stocke une version alternative
juste pour les fichiers concernés, et qu'il y ait un filtre worktree dessus ».

## Ce qui existe, et pourquoi ça ne tient pas

Une tâche (`daemon_host/taches.rs`) ouvre un worktree git complet et **une copie
de la base** du backend (`CopieDeBase`, surcharge `database`), puis sa montre
réindexe le code dans cette copie. C'est le modèle de Git appliqué à l'index :
un arbre entier par branche. Il coûte, par tâche, une base ouverte (son cache,
ses verrous, son journal), une réindexation des fichiers touchés dans un index
qui ne partage rien avec le principal, et un worktree plein sur le disque. À
six PR d'agents par jour, c'est six bases ; à l'échelle d'un service hébergé,
c'est impossible.

## La forme

L'index suit **le contenu**, pas le chemin, et un worktree est **une vue**.

- **Une version par contenu.** Un fichier indexé est une version, identifiée par
  l'empreinte de son contenu (le blob de git, ou l'empreinte du texte pour un
  fichier non suivi). Ses morceaux, ses scopes, ses symboles, ses vecteurs, ses
  entrées de plein texte pendent de la version. Deux branches qui ont le même
  fichier partagent sa version et ses vecteurs, calculés une fois.
- **Une vue est une table chemin → version.** Le principal est une vue ; chaque
  worktree, chaque branche, chaque tâche est une vue : la liste des chemins
  dont la version diffère du principal, et rien d'autre. Une tâche qui touche
  trois fichiers pèse trois lignes.
- **Le filtre worktree est un paramètre de lecture.** Tout outil de lecture
  (search_code, read_file, outline, usages, impact, callees, grep…) prend une
  vue, par défaut la vue du backend appelant ; il résout chemin → version par
  la vue, puis lit la version. Sous L6, la vue fait partie de l'instantané de
  l'appel : une recherche dans une tâche voit la tâche, cohérente.
- **Le drain d'une tâche n'indexe que ce qui change.** La montre d'un worktree
  compare l'empreinte de chaque fichier modifié à celle de la vue principale ;
  une empreinte connue est une version déjà indexée (rien à faire), une
  empreinte neuve crée une version. L'édition par le chat qui code (edit_file
  sur la branche de la tâche) produit une version, une seule.
- **Les relations suivent les versions.** Une arête de code (DEFINES, MENTIONS,
  CONSUMED_BY…) part d'une version ; la résolution d'un symbole dans une vue se
  fait sur les versions de la vue. Les deux sens d'une relation sont écrits dans
  la même transaction, et le relevé ancré des deux sens (hotfixs, 25 octobre)
  devient un témoin de l'index.
- **Une vue se ferme.** Tâche fusionnée ou rejetée : la vue part ; une version
  que plus aucune vue ne cite part avec elle (compte de références), ses
  vecteurs aussi. Une version citée par le principal et la tâche reste.
- **Une seule base, un seul hôte par dépôt.** Plus de copie de base : un backend
  par dépôt, des vues dedans. La mémoire ouverte du poste ne grossit plus avec
  le nombre de branches.

## Ce que ça change pour Git

Le worktree git reste ce qu'il est, l'espace où l'agent écrit et où la personne
relit : il est bon marché tant qu'on n'en fait pas un index. Un jour, la vue
pourra se lire directement dans l'arbre git (chemin → blob) sans checkout,
pour les tâches qui ne font que lire ; ce n'est pas la première marche.

## L'ordre

1. La version comme entité : empreinte, morceaux et vecteurs rattachés ; le
   principal indexé en versions ; témoin : deux fichiers identiques dans deux
   chemins partagent leurs vecteurs (une seule exécution de l'embarqueur).
2. La vue : la table chemin → version, le paramètre de lecture sur les outils,
   la vue par défaut d'un backend ; témoin : une recherche filtrée par vue rend
   la version de la vue et pas celle du principal.
3. La tâche sans copie de base : le worktree + une vue, le drain par empreinte ;
   témoin : trois fichiers édités dans une tâche → trois versions, zéro copie,
   search_code dans la tâche voit l'édition, dans le principal non.
4. La fermeture : compte de références, la vue qui part emporte ses versions
   orphelines ; témoin : la fusion d'une tâche ne laisse rien.
5. Le relevé ancré des deux sens comme témoin permanent de l'index.

Chaque marche est un lot ; la première change le schéma du backend de code et
demande une migration de l'index (une version du format du gabarit, le
manifeste le dit). Sessions : everything declarative (tâches, vues), recherche
(outils de lecture, index), l'arbre principal (L6, résolution).
