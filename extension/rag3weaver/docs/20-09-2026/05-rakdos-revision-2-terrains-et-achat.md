# Rakdos révision 2 : terrain à blessures et achat annoncé

Après accord de l'utilisatrice : +1 Razorkin Needlehead, -1 Magebane Lizard. L'achat du quatrième Razorkin est déclaré par l'utilisatrice et n'est pas présent dans le snapshot ; le JSON le distingue explicitement, sans modifier la collection source. Impression DSK 153 supposée dans l'export, à confirmer au prochain snapshot.

La recherche MCP des terrains avec texte de blessures/perte d'un point a trouvé notamment un Jagged Barrens possédé. Il remplace une Bloodfell Caves : mêmes couleurs noir/rouge et arrivée engagée, mais une blessure adverse à l'arrivée, compatible avec Ob Nixilis et Grievous Wound. Les autres terrains hors couleurs ou infligeant une blessure à leur contrôleur ne sont pas des remplacements équivalents.

Preuves : `experiments/mtga/data/deck-research-2026-09-20/rakdos-lands/`. La première requête combinait incorrectement `must` et `should` au même niveau ; le filtre enum attend une seule clé. Correction en imbriquant `should` dans `must`, puis résultat de 13 impressions. Aucun changement moteur nécessaire.

L'import, le JSON, la validation et le README du dossier `02-le-fisc-de-rakdos` sont mis à jour ; ancienne version conservée dans `revisions/01/`. Vérification du total : 60 cartes, 24 terrains, 14 créatures. Propriété validée par snapshot plus déclaration utilisateur, pas intégralement par snapshot. Pas de nouvelle partie ou import testé pour la révision 2.
