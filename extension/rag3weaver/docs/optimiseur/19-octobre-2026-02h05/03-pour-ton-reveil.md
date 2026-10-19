# Pour ton réveil — ce qui a changé depuis hier soir (G, 19 octobre 2026, 6 h 30)

## Ton hôte d'ici

Rebâti à 6 h 30 sur le master du moment (`9b22ef8d7`) et la lib `courant` (`cf71c929d`). Ton
jeton n'a pas changé : l'URL de `rag3weaver-backend ide` marche comme hier. Vérifié :
- l'hôte tourne par la socket ;
- ta mémoire longue répond avec ses vecteurs ;
- **l'IDE ouvert dans un vrai navigateur, contre ton hôte, en lecture seule**, rend l'accueil,
  l'arbre, un fichier, un symbole, la recherche et le graphe, sans une erreur à la console.

Ce que ton hôte porte de neuf :
- **L'éditeur de l'IDE est CodeMirror.** Il est embarqué dans le binaire : ni CDN, ni téléchargement.
  Ctrl-S enregistre. Si le fichier change sur le disque pendant que tu édites, la page le dit et
  garde ton texte ; l'enregistrement te montre alors la différence et te laisse choisir.
- **Trois défauts de la page corrigés**, que personne n'avait vus avant de l'ouvrir dans un vrai
  navigateur : le symbole, l'impact et les appelés affichaient zéro ; la septième page se figeait ;
  et chaque vue laissait deux erreurs à la console.
- **L'hôte rend sa mémoire** après un index, à la fermeture d'un backend et au repos.
- Les lots de la nuit des autres sessions : le moteur (le ROLLBACK sous tampon plein, les deux
  correctifs du cœur), les relations, l'ambiguïté et les langages de codeparsers.

## Tes manifestes, tranché par c7 — à relire par toi

Ton backend `code-rag3db` servait des outils copiés le 11 octobre : l'IDE t'aurait montré une erreur
dès l'arbre. **Tes deux manifestes désignent maintenant les gabarits livrés par leur nom**
(`builtin:tools/list.mmd`) : ils suivront chaque binaire, sans copie à rafraîchir.
- Rien de ce que tu as modifié n'a bougé : ton `memoire-longue/backends/memory/backend.json` reste à
  toi, et `code-rag3db/.../rag3db.json` aussi.
- Les anciens manifestes sont gardés à côté en `.avant-link`. Chaque copie rafraîchie est gardée en
  `.avant-19-oct` ou `.avant-19-oct-matin`.
- Pour voir ce qui diffère du livré : `rag3weaver-backend templates drift <manifeste>`.

## Deux choses qui sont à toi seule (lucivy)

1. **Un shard de lucivy sans écrivain perd les écritures en silence.** Un ajout est jeté, une
   suppression sautée, et l'appel rend `Ok`. Ticket côté produit, avec les lignes et une recette :
   `docs/tickets/2026-10-19-un-shard-de-lucivy-sans-ecrivain-perd-les-ecritures-en-silence.md`. Le
   correctif est une version de lucivy-core à publier par toi.
2. **Le budget par défaut des écrivains de lucivy est lourd au repos** : 25 Mo × min(cœurs, 16) par
   écrivain, un par shard, quatre shards par table. Sur luciepc, cela fait environ 1,4 Go et 24
   fils tenus, backend ouvert. Je mesure ce matin un défaut sobre posé par l'hôte (2 fils par
   écrivain). Si tu veux plutôt un défaut plus sobre dans lucivy-core, c'est une version à toi.

## À relire si tu veux

- **Le magasin de libs de l'IDE ajoute 1,1 Mo de JavaScript tiers (MIT) au dépôt et au binaire**, après
  réduction : on ne range que ce que le graphe des modules atteint, plus les licences.
- **Ta base de code** peut gagner une réindexation : le lot de codeparsers ôte des arêtes devinées
  que les bases existantes gardent encore.

En cours ce matin : la mémoire des écrivains de lucivy, puis le figement de lucivy dans son commit,
que l'arbre principal a trouvé en mesurant L6.
