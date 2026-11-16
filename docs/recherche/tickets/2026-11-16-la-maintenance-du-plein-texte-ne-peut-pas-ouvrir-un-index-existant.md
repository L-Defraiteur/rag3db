# La maintenance du plein texte ne peut pas ouvrir un index existant

- **État** : ouvert, 16 novembre 2026, demande C7.
- **Pour** : Recherche ; coordination G/Arbre pour copies privées de migration.
- **Famille** : produit
- **Gravité** : maintenance bloquée ; une ouverture ordinaire peut modifier la preuve.

## Constat et liens

Sonde RO rapportée par C7 : `/backend/fts-live-ids`, File, hôte927, renvoie422 avec aucun handle ouvert. Cela ne prouve pas une absence sur disque.
`ensure_fts_handle` et `open_fts_files` suivent un chemin autorisant suppression de dossier sur génération divergente et création après échec d’ouverture : ils ne conviennent pas à cette maintenance.
Suite du [ticket des comptes](2026-11-15-l-etat-d-index-ne-voit-pas-les-orphelins-du-plein-texte.md), prioritaire avant sa qualification. Parent confirmé par Arbre sur sa branche privée `cx-arbre-bascule-bases-20261116-r1`, source `20093817d8` : `docs/arbre-principal/tickets/2026-11-16-basculer-les-trois-bases-vers-les-generations.md`. Le fichier n’est pas présent sur cette base ; lien à rendre relatif après intégration du parent. Aucune base de Lucie n’est ouverte ici.

## Plan avant code

Réutiliser le format snapshot Lucivy (`export_snapshot_sharded` puis `ShardedHandle::open_snapshot`) déjà employé par `fts_handle::open_blob_snapshot`, en adaptant la lecture des fichiers existants sans writer. Auditer la version réellement épinglée avant de retenir cette voie. Ne pas appeler les constructeurs de stockage qui font `create_dir_all`.
L’identité demandée est la table physique dans la cellule exacte du catalogue : aucune résolution/redirection d’une entité publique. Vérifier la génération de base et celle du dossier strictement, distinguer absence, corruption, divergence et erreur IO ; aucun fallback create, rebuild, retrait, warmup ou modification de jeton. Un handle déjà ouvert ne dispense pas du contrat de provenance à définir explicitement.

Publier les témoins avant le correctif : index absent (aucun dossier créé), config/shard corrompu (octets conservés), génération divergente ou illisible (aucun retrait), cellule étrangère et alias public refusés, index existant valide (vivants et jeton cohérents, manifeste avant/après identique). Les erreurs doivent porter des variantes typées, pas une chaîne interprétée. Fixtures privées seulement.

## Limites et qualification

Aucun Cargo/rustc/test Rust lancé ; témoins à écrire puis publier, donc aucun rouge observé à ce stade. Attendre pin G exact avant toute compilation. Inventaire RO d’Arbre distinct de l’implémentation Recherche ; un seul éditeur Catalog. Aucun service/base réelle/purge. Qualification à coordonner sous poste, premier rouge conservé ; aucun ancien GO réactivé.

## Témoins publiés avant implémentation — base68590270

Branche neuve `cx-recherche-fts-existing-only-r2`, W `cx-wt-recherche-fts-existing-only-r1` : aucun produit counts embarqué. Cinq fonctions `catalog::tests::existing_fts_*` appellent la vraie entrée `fts_live_ids` avec index privé Files Lucivy, deux lectures identiques, inventaire intégral avant/après ; cas absent, corrompu, générations divergente/illisible, alias public et autre cellule. Setup writer strictement limité à la construction de fixture, fermé avant observation. Connexion Callback fournit la marque de base, pas un moteur natif ni un décodage simulé. Tous NON JOUÉS ; aucun rouge observé. Refus promesse/erreur IO et validation typée à compléter avant qualification finale.

Recette proposée à C7 : fenêtre CPU/native petite 1200s uniques, compilation témoin plafonnée900s sous poste32GiB/j8/HIGHmax, exécution filtre `catalog::tests::existing_fts_`120s au plus ; premier rouge conservé (attendu sur source avant correctif), pas de retry. Même target Recherche après garde commune, aucun SDK ancien, pin685/lib59ac exact, aucune base/service/modèle. Source rouge figée avant correction ; seconde admission distincte pour vert après revue du diff. Aucun lancement avant accord de recette et slot.

Précision de fixture avant tout essai : `main` est une vraie table dérivée dans ce schéma ; le témoin alias configure explicitement Document.search_index=main et vérifie la résolution avant le refus, puis la retire pour le cas cellule. Aucun verdict exécuté auparavant.

## Premier arrêt de recette — aucun rouge produit observé

GO C7 source187 pour le seul premier positif ; pilote fac5eeee11 arrêté code1 sur collision de nom de portée systemd AVANT superviseur/Cargo. Rapport `docs/recherche/16-11-2026/05-rouge-entree-systemd-fts-existing.md`, log SHA189ee19f784f34e33952cf4ac10079270d810e4c5dd3a4d431893db48b3668a2 identique PC/local. Target/preuves/reçu absents, poste libéré, aucun retry ni implémentation automatique.

Réserve C7 avant implémentation : une table physique Document reste admissible même avec search_index=main ; le chemin maintenance doit ouvrir exactement Document, jamais rediriger ni refuser au seul motif de ce mapping. Remplacer le négatif187 par nom réellement inconnu et ajouter Document41/main99 comme positif distinct ; autre cellule toujours refusée.

Correctif du lanceur et témoins publiés78437ce0a6 : suppression scope redondant, POSTE_SWAP_MAX=0 en plus32G/800%, driver recontrôle high/max/swap/quota. Deux fonctions hermétiques/7cas vertes0.019s après publication ; aucun Cargo/reçu réel nouveau. Source187 figée, attente nouveau GO distinct C7.

## Rouge comportemental réellement observé avant correctif

Source187 figée, GO distinct C7 et pilote3a528b67921f3c3b698e6d6db593bf8d4f834516 : compilation125.019s réussie puis **unique premier positif0/1/0** sur absence de handle ouvert Document, pas sur un préfixe nouveau. Rapport06, preuves25 fichiers PC/local réhash0écart manifestea3519cc5325b675e7d4c4eff3c4d9061aa380175a8971ee1ced34fcbfa2b8110. Confineur ROUGE101/restes[]/nettoyé/reçu terminé, poste libéré, aucune autre fonction ni implémentation automatique. Target de session gardé. Quatre autres témoins NON JOUÉS ; leur réserve physique/public reste à corriger avant full.

## Rouge pending distinct — prérequis non levé

C7 a demandé le témoin préfixe1f816 seul avant le fixpending. Pilotec306, compilation29.058sOK, unique0/1/0 MAIS échec de fixture avant snapshot : main n’est pas une dérivée par contenu admissible de Document. Rapport08, archives26PC/local0écart manifest19c174ee157c7720299fba7560005fcb4b777572f6b18b25b66c2a315ea2bcdb. PremierrougeSTOP/reçu terminé/restes[]/poste libre, target conservé, aucunretry/qualificationa92. Corriger fixtureContent/from/render/champscompatibles ET juges utilisant ce mapping avant nouvelle admission distincte ; aucun rouge pending observé pour l’instant.
