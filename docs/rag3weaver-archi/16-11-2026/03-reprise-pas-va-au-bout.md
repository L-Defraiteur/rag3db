# Reprise du lot « pas va au bout », préparation seule

Référence auditée après fetch : master927941a67a5805450d49f1e04b39fdd542a6059b. Reprise autorisée par C7 après audit des différences récentes, sans passe pendant la fenêtre de bascule G/Arbre. Arbre de Lucie intact.

## Candidat

Branche neuve `cx-archi-pas-va-au-bout-r1`, worktree `/home/lucied/.cache/rag3weaver-build/cx-wt-archi-pas-va-au-bout-r1`. Deux patches de `pas-va-au-bout-2` (tête5e25a110d7747531bc1c006030f1262c397d0775, base6b672afa6) rebasés sans conflit sur927 : rouge historique `e0f084f4e2`, correctif `3ae1e741448b70e70e4d9ebbd43ceebf3a80da2a`. Range-diff2/2 strictement égaux. Chaque commit est publié sur origin, ancienne branche et W conservés.

Deux seuls fichiers produit réservés : `src/daemon_host/teammate.rs` et `teammate_tests.rs` dans la crate. Ticket : [contenu demandé absent](../tickets/2026-11-14-l-equipier-rend-un-compte-rendu-au-lieu-du-contenu-demande.md). Aucun changement de backend/chat/policy, ni S1/S2 dans ce lot.

La réponse sans step_done est copiée seule avec origin host_copy ; le pas suivant repart avec la demande de continuer, sans attendre le superviseur. Les chemins question/approbation et stopped_by restent prioritaires ; MAX_STALLED/MAX_STEPS bornent la suite. Les témoins relus jugent la continuation et la forme de la copie, pas la réalisation physique d'un test que le faux modèle dit avoir écrit. La campagne D6 après livraison juge le parcours réel.

## Ce qui est acquis, ce qui ne l'est pas

Ancien lot : témoins rouges puis passe annoncée verte sous lib373b5477b (module62, lib2531/0/72, navigateur45, hooks, Vertex2 ; rejeu court sur6b672afa6). Les journaux bout-module/bout-pilefile/bout-e2eseul restent historiques. Aucun de ces résultats ne devient un verdict sur ce nouveau candidat et la lib neuve.

Depuis6b672afa6 jusqu'à927, seuls Cargo.toml/Cargo.lock changent le produit entrant : pointeur lucivy4.3.9 cb4a1757a4. Pas de conflit sur les deux fichiers réservés. Nouveau calcul NON JOUÉ ; aucun target/SDK/service posé par cette préparation. Les tickets S1 budget12, S2 frontière sans retours et S3 request restent ouverts ; garde B/socle/nom-persona-modèle sont déjà sur master et ne se rejouent pas comme nouveaux développements.

## Recette proposée après FINI G uniquement

1. Recevoir de G/C7 le FINI de la bascule et l'identifiant COMPLET de la lib neuve attendue59ac, le hash et la version des scripts de passe/garde déployés. Relire master et ses entrants : nouveau code Rust impose nouvelle branche et qualification adaptée. Aucun ancien GO ni délai ne reprend.
2. Préparer un target privé neuf et des journaux privés neufs, sous timeout frais couvrant attente poste et suites, POSTE_MEM_MAX32G/HIGHmax/j8 ; une portée commune et bus privé. Module `daemon_host::teammate` et `daemon_host::chat`, puis check_features, e2e_demon_hote, e2e_lectures, e2e_equipier sous Vertex réellement joué (jamais skip vert). La lib entière, hooks et NAV45 sont inclus si requis par la garde/entrants/bascule ; le jeu exact et la borne seront figés avant lancement avec C7, pas inventés depuis les durées de l'ancienne lib.
3. Employer la vraie passe épinglée/les entrées gardées pour produire résumé et base ; ne pas fabriquer un resume.log depuis des morceaux d'anciens journaux. Le script `tools/poste/passe_epinglee.sh` de927 force encore RAG3DB_LIB373b5477b : exporter une autre valeur en amont ne le change PAS. Attendre sa version épinglée par G/C7 ; ne pas éditer la copie du cache ou contourner la garde. De même, `wt-bout-passe.sh` reste un modèle ancien à relire, pas une commande autorisée telle quelle.
4. Vérifier les noms des témoins, absence de nouveaux skips et refus réels, hash source/ELF/lib, premier rouge STOP, journaux conservés. Rapporter prêt à C7. Le push de code master exige son nouveau « à toi », puis dernier fetch/entrants/garde et trois contrôles. D6 est coordonnée par C7 après lot équipier.

Aucune lecture de clé Vertex : seul son chemin est transmis au moment autorisé. Aucun registre pair, PR42, pose d'hôte ni nettoyage. S1/S2 viennent ensuite ; backend.rs/chat.rs ne sont réservés qu'après accord de fichiers avec ED. Préparation actuelle terminée, attente FINI G et recette exacte, pas un verdict VERT de qualification.
