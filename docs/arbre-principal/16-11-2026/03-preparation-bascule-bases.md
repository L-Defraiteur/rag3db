# Préparation de la bascule des trois bases, 16 novembre

Autorisation Lucie relayée par C7 : priorité lib/pose par G, migrations des bases par arbre APRÈS FINI de G, oracle par banc. Pas de nouvelle permission routinière, mais les faits de qualification manquants sont des conditions techniques, jamais un vert déduit. A1 suspendu ; aucune modification Rust/C++ du produit. Aucun ancien délai ou GO réactivé.

## Pins et identités préparés

- Sourceaudit/hôtes en rebâti :927941a67a5805450d49f1e04b39fdd542a6059b, lock Lucivy4.3.9 ; G exclut les anciens binaires3177/373b.
- Lib prévue : `rag3db-build-59ac9c86bc2b3529b590a7462ab7051e3ad26b8a-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF`. Les SHA ELF/libvector/BUILD_ID effectifs seront pris dans les preuves de G, pas inventés avant FINI.
- Base portable : `/home/lucied/.local/share/rag3weaver/code-rag3db-depot.rag3db`, manifeste source `~/.config/rag3weaver/code-rag3db/templates/backends/code/rag3db.json` (SHA8ebbb2e34b582207276980bb0a84457aeaf6e3b5daaf8893f6a3aa2488a2cc77).
- Bases PC sous même répertoire : `code-principal.rag3db` et `code-lourd.rag3db`, sources `~/.config/rag3weaver/<nom>/templates/backends/code/backend.json`, SHA329a51c5f88595074e2eecebcffccea1aad5be2dce1cfa5d3cdbba50abf20fec et3e46a86a510cefe0f37cdaef6012f7ab4fa9257e30617b97f2a20c8f66eb7420.
- Exports sélectifs sans secrets de G : `~/.cache/optimiseur-notes/bascule-16nov-927/manifest-paths-{portable,luciepc}.json`, sur les postes correspondants. Revalider ces SHA à l'entrée ; aucun manifeste vivant édité par arbre.
- Répertoire proposé par poste pour preuves : `~/.cache/rag3weaver-build/bascule-live-20261116/`, un sous-dossier neuf par base ; copies, journaux et identités hors arbre de Lucie. Pas encore créé à cette préparation.

## Recette et responsables, avant toute entrée

1. G finit sa lib/pose (hôtes/sockets/services/tunnels/courant exclusivement à lui), ses instantanés PRElib qui incluent la phase avant purge, ses contrôles deux sens. FINI doit nommer source exacte, binaires et libvector/lib/SHA/BUILD_ID réel, chemins et identités des snapshots/RETOUR.
2. Avant purge, relevé recherche et inventaires par l'hôte 4.3.9, sur la base qu'il possède. Maintain par table : relevé token des ids plein texte d'abord, lecture graphe ensuite, purge bornée max_purge_ratio0,01 + compact, aucune liste ancienne réutilisée. Refus de jeton/purge horsborne arrête, pas élargissement du seuil. Le snapshot PRElib de G protège aussi cette phase.
3. G tient le backend fermé et prouve l'absence de détenteurs. Snapshot migration BASE+wal+extensions+fts/checkpoints ensemble et RETOUR, provenance lib/bin/config conservée72h. Deux sens frais obligatoires ; réutiliser un verdict G seulement si snapshot/base/identités et absence d'écriture graphe intermédiaire le justifient explicitement. Sinon rejouer contrôle avant1b.
4. Sous nouveau `poste lourd timeout`/32GiB/HIGHmax/j8, appel du binaire neuf épinglé `migration versions --base BASE`, LOT2000 existant ; puis `migration generations --base BASE`. Base verrouillée/refus/erreur partielle = arrêt et conservation, jamais rejouer aveuglément. Bilan attendu versions→generations,backend:0publiée/ranglu1. Aucun ancien index Scope_Chunk/Lucivy_Scope retiré.
5. G rouvre le backend. Index confirm=true par l'hôte neuf, attendu jusqu'à sa DERNIÈRE ligne DONE_LINE, échec/stop et dette réelle enregistrés ; busy n'est pas la fin. Repassage base ouverte, ne pas modifier son manifeste, ne pas lancer un second écrivain autonome sur la même base.
6. Contrôles : formes/estampille, comptes bruts ET visibles par table distincts (File et arêtes d'histoire doublent légitimement), miroirsCONSUMES/CONSUMED_BY dans les deux sens0/0, modèles/colonnes/vecteurs actuels et dette, outils de recherche effectifs. La disponibilité vector+bm25 doit être observée, jamais inférée d'une configuration ou d'une réponse vide. Juger visibles à la même couche/rang, ne pas confondre publié et attente.
7. Banc reçoit copie finale et même état de sources (REGLES7/codeparsers, non-suivis compris, manifeste/modèle identiques). Oracle bonnesréponses séparé de parité ; scores BM25 à5e-3 relatif, vecteurs cos>0,99999/maxabs<1e-4 et stabilité rangs selon contrat existant. Un ancien Bprime au seulcommit sans non-suivis n'est pas cet oracle. Une copie/lien absent = NONQUALIFIÉ, ne pas promettre égalité entre corpus différents.
8. Succès ou refus de chaque base documenté séparément. Retour72h garde ancien index/snapshot/config/bin/lib ; pas de nettoyage. En cas d'écart avéré, coordination du retour avec G (services à lui) ; restauration sans effacer preuves de l'état après. Ne pas attendre un nouveau mot Lucie pour avancer une étape déjà autorisée et qualifiée.

## Outils et limites de préparation

Le script PC historique `bascule/repetition-bascule.sh` est lu, pas exécuté : il épingle req373b/PURGEancien/bin794 et ses manifestes étaient pour des COPIES. Le vieux requete.cpp portable ouvre en écriture, celui PC tronque30lignes et rend0mêmeerreur : aucun ne vaut outil de preuve inchangé. Nouvelle petite sonde privée `bascule/read-only-query.cpp` : readOnly moteur, tampon1GiB, toutes les lignes, erreurs nonzero, BUILD_ID lu dans le moteur et dit par binaire. À recompiler par G contre59ac (pas un binaire ancien réutilisé) puis hasher. Elle ne sert pas à lire une base vivante détenue par l'hôte : contrôles à froid après release par G, ou sur copies.

Aucune compilation ni ouverture de base par cette préparation. Conditions encore manquantes : FINI G et SHA/outils compilés, copies finales et état source oracle, créneaux réels selon mémoire/services. Capacité réelle décide si trois bases parallèles ou séquentielles. Durées historiques communiquées G : ~15min/baseoffline HORS2sens(~20minportable/~40PC dans sa fenêtrelib),45–60minrepassagebaseouverte,~13minoracleindexneufPC enplus ; rien ne garantit ces durées sur nouveaux binaires et charge.

## Préflight sur fixture privée et précision oracle

Premieroutil04a compilé parG contre59ac : rouge BUILD_ID sansRETURN sur fixture1nœud (aucune basevivante). Requête corrigée CALL BUILD_ID() RETURN *, à rejuger avant usage. Autres lectures du binaire restent readOnly/erreursnonzero/pas de troncature.

Le seuil BM255e-3 ne s’applique qu’aux scores BRUTS Lucivy sur mêmes contenus/ids visibles ; jamais au RRF de search_code. Recette oracle banc exacte df02f39a06c9949d285acd08747824f30cfa0297 lue, lecteur927/59ac encore à qualifier parbanc aprèsFINIG. Maintenance possible via `/backend/maintain` dry_run : primitive `fts_maintenance::releve_a_blanc` prend les live_ids et leur token AVANT lecture graphe et revérifie après ; réponse orphans/token ensuite transmise àmaintain max_purge_ratio0,01/compact. Aucun outil373/PURGE4.3.8 requis. G seul ferme/ouvre services.
