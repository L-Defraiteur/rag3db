# Candidat d’intégration du raccord physique

Branche neuve cx-arbre-raccord-integration-r3, rebasée depuis le paquet de
preuves0f260599a sur masterfa4dc35001c4b86494ab0b9954cb6e2d2d619f0d.
Deux commits entrants depuis f8 sont exclusivement Markdown (ED dette read_scope
et Optim clôture de la fenêtre card0). Aucun nouveau runtime requis par C7.

Audit [JSON](raccord-physique/integration-r3-audit.json) et
[range-diff](raccord-physique/integration-r3-range-diff.txt) :36 patches égaux,
dont22 code ;6779 entrées produit suivies byte-identiques à
4e5ae8a324f50313b74f70772eb2e92ee41fad6f (tout hors docs/, incluant tests,
gabarits, gitlinks et lockfiles).7871 chemins master hors chevauchement
préservés, suppressions master préservées. Les clôtures et cet audit sont
uniquement documentaires après le rebase.

Qualification composite : préparation source4e et runtime correctif pilotea18,
preuves0f260599a, [rapport07](07-preuves-globales-correctives-physiques.md).
Les rouges historiques restent rouges. Trois revues indépendantes des preuves
favorables (Recherche, Mémoire, ED) avec périmètres déclarés ; C7 favorable.
Aucun claim d’absence GPU : FD dri observés. Aucun index réel/legacy/pyramide
ou projection de coordonnées fichier qualifié. Aucun DROP/retrait autorisé.

Trois tickets déplacés par git mv vers docs/tickets/closed : tables UUID,
lecteur de blobs RO, pilote cwd/profil. États corrigés avec sources et preuves
composites, limites et historique conservés. Parent ancres reste OUVERT pour
la portée restante. Aucun GO master déduit ; candidat publié pour revue et
tour explicite C7 avant poste push/ifVERT/trois contrôles.
