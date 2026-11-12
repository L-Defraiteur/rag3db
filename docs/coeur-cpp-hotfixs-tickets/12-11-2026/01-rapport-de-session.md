# Hotfixs — point de reprise du 12 novembre 2026

## Diagnostic de quarantaine

Porteur hotfixs avec cœur C++ ;
[ticket ouvert](../tickets/2026-11-12-verifier-la-quarantaine-d-une-annulation-partielle-pour-les-transactions-deja-ouvertes.md).
[Résultat structurel](04-resultat-structurel-quarantaine.md) : après annulation
partielle et libération des verrous TA, les commandes publiques refusent la base
mais un COMMIT forcé déjà ouvert atteint la fabrique interne de checkpoint.
Sentinelle avant Checkpointer ; aucun checkpoint réel ni réouverture. Ni
corruption ni innocence générale démontrée. Compilation01 puis runtime01 ont
été arrêtés sur erreurs de fixture et conservés ; corrections précises relues
et autorisées avant leurs reprises. Aucun retry automatique.

Lot `cx-hotfixs-temoin-quarantaine`, worktree
`/home/lucied/git_workspaces/cx-hotfixs-temoin-quarantaine`, source témoin8cc47439f.
Commits sauvegardés sur leur branche distante, aucun tour master/code produit.
Aucune compilation ou exécution supplémentaire à lancer implicitement.
Les bases et binaires privés restent à luciepc, chemins et quatreSHA dans
le résultat. Ticket ouvert pour décider du contrat et des suites avec cœur.
Ancien lot de recette documentaire `cx-hotfixs-recette-quarantaine`/32fc739a8
conservé sans retrait ni rebase ; son plan général n’autorise aucune variante.

## Service privé7882

Coordination liée au [ticket banc dix fiches](../../banc/tickets/2026-11-11-mesurer-le-juge-local-avec-dix-fiches.md),
porteur banc et opérateur optimiseur. Usage transmis à codeparsers le11nov,
puis codeparsers libéré selon orchestration. Remise temporaire de gestion à
optimiseur confirmée pour fenêtrebanc20min ; arrêt SIGTERM ciblé PID912225 et
restauration seulement au GO de l’orchestration/banc, aucun autre service.
Aucun arrêt ni nouvelle chargeGPU par hotfixs dans cette coordination.

Lecture de préparation : service /sante OK granite278m/dim768/Flex32,
aucun clientTCP7882 ; exe processus=installé SHA
`65ec1d1db17d48ee53785cd55e72b9b77ca451d5bc4bb8197a9c1b7d040c595d`.
Cwd `~/.cache/rag3db-mesure-g5/embed-7882`, commande
`~/.local/bin/rag3weaver-embeddings --adresse 127.0.0.1:7882` ;
env observé limité aux quatre réglages pertinents :
`RAG3WEAVER_BURN_DEVICE_EMBEDDER=gpu:0`, `RAG3WEAVER_EMBED_MODEL=granite-278m`,
`RAG3WEAVER_EMBEDDINGS_ADDR=127.0.0.1:7882`, `RAG3WEAVER_REGIME=plein`.
FDamdgpu PCI0000:04:00.0/renderD129 porte les allocations, même PCI que card2 ;
i915 PCI00:02.0 ne porte aucune mémoire/moteur actif dans le relevé.
Optimiseur doit revérifier le ciblage physique après restauration ; gpu:0 n’est
pas une preuve indépendante de card2. Aucune utilisation card0/seat1 autorisée.
