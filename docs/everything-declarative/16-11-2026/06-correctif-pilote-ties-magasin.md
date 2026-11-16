# Pilote ties correctif : magasin explicite et reste de la borne initiale

Ticket [rouge du magasin](../tickets/2026-11-16-le-pilote-ties-isole-le-cache-sans-epingler-le-magasin-de-libs.md)
publiébbe495 AVANT correctif. APPbd76 inchangée. Contrôle séparé branche
neuve `cx-ed-stable-tie-qualifier-r2`, P/Q/Sv2 neufs. La v1 et ses preuves
restent conservées ; aucune autoritév2 créée à publication.

Le profil du driver fixe RAG3DB_LIBS_DIR au magasin R.parent/rag3db-libs
canonique vérifiéUID et deux hashbins. XDG_CACHE_HOME reste privé. Aucune
copie/lien de lib dans ce cache ; lib59ac complète/bibliothèque et extension
vector épinglées comme v1. La fonction réelle `library_environment` est
extraite par le témoin `test_library_environment.py`, qui appelle le VRAI
`lib_du_magasin.sh/exporter_la_lib` et son vérificateur de planchers : sans
magasin explicite sous XDG privé, refus2 attendu ; avec profil corrigé,
cheminsLIBRARY/INCLUDE/LD exacts etSHARED1. Aucun Cargo, base, dlopen ou
fichier dans le magasin n'est requis ou créé par ce test. **NON JOUÉ à ce
commit de publication ; code et témoin sont publiés avant leur exécution.**

Admission dans `admit.py` APRÈS vrai poste :
`floor(1180927.964331921 - monotonic_now) - 3`, refus<=60. START reste
ENDoriginal−7200 ; driver impose AUTHEND<=ENDoriginal et les caps/contrôles
intersectent ce reçu. Finaliseur relit les deux hashbins puis compare END et
duréeoriginale avant publication composée. Aucun reset7200.

Target même sourceEDr2 : marqueur exactC, UID/canonique, zéro .fingerprint
attendu puisque v1 a refusé avant Cargo ; reçu v1 terminé, scopeROUGEpropre,
configAPPbd76, premierrouge et log exact0 contrôlés avant admission. La
garde commune est appelée réellement sur ce même propriétaire, aucune
purge ni réadoption étrangère. Les copiesPQS sont neuves et leur manifeste
prepared.json garde aussi les empreintes du rouge et de la configv1.

Les deux ciblés et la passe canonique sont inchangés. Qualification produit
toujours NON JOUÉE ; les2595 verts du matin ne s'y transfèrent pas. Le GO
correctif et le même créneau ont été coordonnés par C7 ; RO du couple exact
et contrôle du reste avant départ, sans nouvelle permission de Lucie.
