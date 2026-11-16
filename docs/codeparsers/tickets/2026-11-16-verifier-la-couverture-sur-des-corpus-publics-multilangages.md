# Vérifier la couverture sur des corpus publics multilangages

- **État** : ouvert ; mission demandée par Lucie, préparation et acquisition autorisées, mesures à borner.
- **Famille** : produit
- **Gravité** : qualification de la couverture des langages moins exercés, sans défaut précis présumé.
- **Atteignable en service** : indirectement, qualité de l'analyse de code ; aucun accès à une base vivante dans ce banc.
- **Touche rag3weaver** : oui, bibliothèque codeparsers et chemin d'analyse employé par l'indexeur.
- **Pour** : codeparsers (analyse et diagnostics), banc (corpus et reproductibilité), orchestration (ressources et suivi).
- **Signalé par** : Lucie, le16novembre2026 : télécharger des dépôts d'exemple de chaque langage, environ2à3kdocuments, tester le parsing et vérifier la couverture des usages moins habituels.

## Périmètre

Interpréter les documents comme fichiers sources du langage visé, avec les variantes syntaxiques pertinentes. Le routage actuel de parse_file/EXTENSION_TO_LANGUAGE expose huit catégories : Rust, Python, Go, C, C++, C#, JavaScript/JSX et TypeScript/TSX. Les noms supplémentaires dans une énumération générique ne prouvent pas l'existence d'un parseur. Confirmer cette liste au commit mesuré, distinguer texte brut et extraction sémantique. Les .h sont routés C++ actuellement : ne pas les compter silencieusement comme du C.

Ne pas interrompre ni dupliquer le correctif std/homonyme7601 déjà vert sur deux régressions. Banc indépendant, sources et pilotes épinglés ; aucun changement des parseurs ou du routage pour embellir le résultat.

## Corpus reproductibles

- Dépôts publics significatifs, environ2000à3000 sources par catégorie lorsque leur structure s'y prête ; un sous-projet cohérent ou plusieurs dépôts complémentaires valent mieux qu'un quota artificiel de fichiers.
- Manifestes avec URL, SHA complet, licence, sous-arbres, règles d'admission/exclusion, tailles et SHA des fichiers retenus. Conserver contexte et manifestes de projet pour la résolution ; signaler toute cible absente de la sélection.
- Séparer production, tests valides, données volontairement invalides, généré et tiers. Ne pas cacher une exclusion, un fichier trop gros, une erreur de lecture ou un dépassement de délai.
- Téléchargements autorisés pour ce travail ; ne pas exécuter les scripts de build, installations ou hooks des dépôts externes. Aucun secret, modèle, GPU, hôte7970 ou base de Lucie requis.

## Mesures et preuves

Réutiliser d'abord les outils de mesure existants. Distinguer fichier découvert, admis, réellement analysé, fallback texte brut, erreur AST/MISSING, panne, délai, zéro scope légitime ou extraction anormalement vide. Mesurer scopes/définitions/imports/relations et bornes de positions, par famille syntaxique lorsque le parseur le permet. Des captures absentes ne sont pas une réussite parce que le parsing n'a pas paniqué.

Vérifier un échantillon explicite de définitions et relations : classes, méthodes, génériques, imports, héritage, fonctions libres et cas propres à chaque langage. Séparer diagnostic de syntaxe et exactitude sémantique ; indiquer l'oracle et ses limites. Les compteurs seuls ne prouvent pas un rappel de100%. Signaler les cibles hors corpus et dépendances non disponibles, sans les confondre avec des régressions du résolveur.

Compiler notre outil et exécuter les corpus sous vrai poste et portée commune bornée32GiB, quotas par place, au plus8jobs, aucun -j(nproc). Fixer le commit parser et celui du consommateur, le hash ELF, l'inventaire et les paramètres avant mesure. Comparaison de coût annoncée seulement au poste mesure exclusif ; durées observées sous concurrence ne sont pas un banc de performance.

## Fin et suite

Publier tableau par langage avec dénominateurs, erreurs, cas vérifiés, couverture prouvée et inconnues ; conserver corpus figés/manifeste et preuves réexécutables. Toute lacune confirmée a son ticket fils et un exemple minimal, avec rouge avant correctif. Ce ticket n'autorise pas une série illimitée de nouveaux parseurs : les langages non pris en charge et extensions futures sont signalés séparément. Fermer uniquement lorsque les corpus et résultats demandés sont réellement livrés ou que le périmètre est explicitement abandonné.
