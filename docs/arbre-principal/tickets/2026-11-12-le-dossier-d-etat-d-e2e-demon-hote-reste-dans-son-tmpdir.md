# Le dossier d'état d'e2e_demon_hote reste dans son TMPDIR

- **État** : ouvert, découvert pendant la passe diagnostique migration v2 ; attribution baseline à vérifier avant correction.
- **Famille** : produit
- **Gravité** : passe refusée par le garde de résidus, malgré les assertions fonctionnelles vertes.
- **Atteignable en service** : non, hygiène du lanceur et des fixtures e2e.
- **Touche rag3weaver** : oui, état temporaire des témoins et nettoyage de suite.
- **Pour** : arbre-principal pour diagnostic dans le lot courant ; revue orchestration/recherche pour le propriétaire du nettoyage.
- **Signalé par** : passe sur3377cdd84890770702fe9201c26cf457b54f24ce, liée au [ticket migration](2026-11-12-migrer-les-ancres-de-code-vers-le-contenu-partage.md).

## Constat et preuves

`cx-passe-preflight-1b-v2/e2e_demon_hote.log` : **13 réussis,0 échec,0 ignoré**,
puis rc101 du garde :24Ko dans
`rag3weaver-etat-des-e2e-1475162/manifestes/{70b6c70193074b4b,29b9297f6a44be3a}`,
avec livre.json et premier-index. La suite n'est pas dans la liste de résidus
connus ; aucune exception n'est ajoutée. Dossiers conservés par le lanceur :
`/home/lucied/.cache/rag3weaver-build/e2e-gardes/e2e_demon_hote-20261112-131052-1474799`.

La passe s'est arrêtée à cette étape : features5+minimal verts, lib2152/0/64,
navigateur39/0/0 et migration_preflight5/0/0 verts ; dix autres suites non jouées.
Portée terminée et poste0/4 relu. Ces résultats ne constituent pas une passe verte.

Le [ticket fermé du9nov](../../tickets/closed/2026-11-09-les-temoins-ecrivent-leur-etat-de-manifeste-dans-le-dossier-d-etat-reel.md)
a corrigé l'écriture dans l'état réel : le constructeur de binaire e2e place
RAG3WEAVER_STATE_DIR sous son TMPDIR. Il ne traite pas la fin de vie de ce dossier.
Ce ticket ne rouvre pas cette correction ; il suit le résidu du dossier désormais
privé. Le produit conserve légitimement son livre de manifestes après fermeture.

## Diagnostic et critère de fin

Comparer e2e_demon_hote sur la baseline masterb28 avec mêmes features/lib/lanceur,
sans retry aveugle, puis attribuer le responsable de la durée de vie. Le nettoyage
doit porter uniquement sur un état de fixture dont le lanceur possède la racine ;
conserver cet état en cas d'échec. Pas de nettoyage du dossier d'état réel, pas
d'affaiblissement du contrôle de résidus ni simple ajout à sa liste d'exceptions.
Un résidu arbitraire non possédé doit continuer de faire rougir le garde.

Fin : témoin de nettoyage et de garde, puis suite hôte entière réellement jouée
et TMPDIR vide. Documenter source/compteurs/propreté ; revue avant reprise de la
passe migration complète. Aucun transfert, modèle, serveur ou base réelle ajouté.

## Attribution baseline

Contrôle autorisé sur master**b28b55db6a8e79e149e2c3943b05b0cf258d1dec**,
mêmes huit features/lib29/runner et environnement sans services, sous lourd32Go,
j8, borne900s : **13/0/0**, puis **même résidu24Ko** du dossier d'état
`rag3weaver-etat-des-e2e-1530516/manifestes/` et rc101 du garde.
`cx-diagnostic-residu-hote-baseline-b28-v1/{suite.log,resultat.json,source.json}`.
GARDER=1 conserve les dossiers propres à ce diagnostic (pas d'effacement de preuves
antérieures) : `/tmp/rag3weaver-e2e-1527916` et
`/home/lucied/.cache/rag3weaver-build/e2e-1527916`. Aucun retry de passe complète.
Ce défaut de fin de vie préexiste au préfixe de migration ; les deux écritures
dans l'état privé des fixtures sont légitimes, leur propriétaire ne les nettoie pas.

## Correction proposée, sans modification produit

Le lanceur peut créer par mktemp un espace d'état privé à la suite, sous le
TMPDIR qu'il possède, le passer explicitement par RAG3WEAVER_STATE_DIR, puis le
retirer après succès avant de vérifier les résidus non possédés. En cas d'échec,
le garder avec les dossiers diagnostiques. Ne jamais déduire un chemin à effacer
de l'environnement d'état réel ; ne supprimer que le chemin créé par le lanceur.
Le test d'isolation sous TMPDIR reste vrai et les livres du produit persistent.

Témoins d'outillage requis avant feu vert : état possédé seul nettoyé ; fichier
arbitraire hors de cet espace toujours rouge ; état conservé sur échec. Puis le
vrai e2e_demon_hote13/0/0 avec TMPDIR vide. Source/validation à relire avant une
nouvelle passe migration, aucun ajout à la liste d'exceptions de résidus.
