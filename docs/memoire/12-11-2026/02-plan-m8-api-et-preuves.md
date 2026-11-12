# M8 — première tranche de plan sur contenus fournis

Ticket : [M8 ouvert](../tickets/2026-11-07-une-memoire-ne-sait-pas-que-ses-gabarits-sont-en-retard.md), porteur mémoire. Autorisation et frontières : [répartition des suites](../../tickets/closed/2026-11-12-paralleliser-les-suites-autorisees-sans-chevauchement.md). Référence de conception : [revue canonique](01-relecture-m8-adoption-fusion-et-reprise.md). Ce document suit le code en branche avant ciblés verts ; il ne déclare aucune livraison master ni droit d’application.

## Contrat implémenté en branche, à vérifier

`fusion::atomic` et `fusion::json` calculent la fusion base/local/cible. Le JSON descend seulement dans trois objets ; absent et null restent distincts, tableaux et fichiers sont atomiques, suppression/modification et conflits de forme refusent au chemin concerné. Les conflits sont déterministes et aucun assemblage textuel implicite n’est effectué.

`template_plan::Package` fournit explicitement des fichiers possédés, les contenus de références builtin et une empreinte vérifiée de cet inventaire. `declared_provenance` reste déclarée : aucun commit Git n’est résolu ou certifié. `plan(bases, local, target, manifest_name)` exige une base unique, des chemins non ambigus et les sources nécessaires ; il n’acquiert rien.

Le résultat `Plan` porte les identités/provenances déclarées des trois sources, les contenus candidats, les fichiers ajoutés/retirés/modifiés et le diff des outils. Le champ des attachements modifiés ne prétend pas couvrir tous les changements transitifs de comportement : les changements de fichiers sont rendus séparément. Aucun renommage d’identité ni inventaire exhaustif d’appelants n’est déduit. Aucun journal n’est lu.

Les refus sont structurés : base absente/ambiguë, empreinte incompatible, source/chemin invalide, conflit, compatibilité à qualifier et validation commune. La première tranche refuse conservativement les changements des déclarations d’entités/relations, base/racine de fichiers, modèles/signaux et workspace, ainsi que les schémas modifiés, même pour un ajout facultatif. Les références externes de JSON Schema ne disposent pas encore d’un résolveur fermé commun : elles sont refusées avant compilation, pas recherchées sur disque/réseau. Ces limites ne constituent pas un second moteur de migration.

## Primitives communes et absence de repli

`lecteur::Contenus` est un univers fermé sur fichiers et builtin fournis. Son identité de chemin est logique, sa liste de dossiers est dérivée des sources, les références manquantes refusent. La capacité de vérification physique de l’environnement refuse sans exécuter le contrôle. `Disque` et le recouvrement ordinaire conservent leurs chemins existants.

Le chargeur commun utilise ces identités/résolutions, y compris pour l’état d’écran, les schémas, les plans et les sous-graphes cités. `builtin_templates::resolve_avec` reçoit un résolveur ; la voie ordinaire garde `root`, la voie fermée ne l’appelle pas. Le raccord de `foreach_node` délègue les références builtin du candidat au même lecteur ; le contenu du binaire ne remplace pas silencieusement l’instantané fourni. La voie publique historique de résolution des sous-graphes garde son comportement ordinaire.

Les vérifications Landlock du chargement ordinaire restent exécutées. Dans l’univers fermé, une preuve physique non disponible provoque un refus explicite. Aucun environnement manquant n’est rendu valide par un flag permissif. Le candidat passe le même chargement déclaratif ; aucune base n’est ouverte et aucun modèle connecté par le plan.

## Périmètre de fichiers annoncé et confirmé

Nouveaux `src/fusion.rs`, `src/template_plan.rs` et déclarations dans `src/lib.rs` ; raccords `src/lecteur.rs`, `src/builtin_templates.rs`, `src/backend.rs`, `src/dataflow/foreach_node.rs`. ED possède la correction des accords dans `commande.rs`/`run_nodes.rs`, pas ces raccords ; confirmation reçue avant modification des fichiers communs.

## Preuves réellement obtenues à ce point

- Montage sans natif : compilation E0432 avant tout test, pas un rouge M8. Il est suivi dans [le ticket tty canonique](../../tty/tickets/2026-11-07-la-lib-rag3weaver-ne-se-batit-pas-sans-rag3db-native.md). Le doublon mémoire est clos comme remplacé. Le cfg minimal `80faa4838` reste un commit séparé, sans sources M8.
- Rouge M8 observé à luciepc sous poste lourd, source `59b3f0ff7`, lib29 épinglée et target privé `cx-target-memoire-m8-plan` : **0 passé, 1 échoué**, aucun ignoré, 1610 filtrés. Un processus neuf appelle le chargement existant avec un candidat fourni ; le refus extrait les gabarits dans son cache temporaire vide. Cause dans `cx-memoire-m8-rouge-natif.log` : « le refus de la proposition a extrait les gabarits ».
- Les verts du nouveau code ne sont pas encore joués. Le témoin froid est raccordé à la voie fermée, sans relâcher l’assertion d’absence de cache ; la voie disque ordinaire n’est pas renommée pure.

## Ciblés proposés avant revue du diff

Fusion : éditions disjointes/identiques, absent/null, suppression/modification, formes et tableaux. Lecteur : identité logique, fermeture au disque, ambiguïtés et refus d’un contrôle physique sans l’exécuter. Plan : candidat complet, empreintes/bases, ressources manquantes, builtin fourni différent du binaire, sous-graphe transitif, schéma modifié/refus de référence externe, conservation d’un no-op et diff déterministe. Processus neufs pour refus à cache froid et candidat valide malgré un cache physiquement inutilisable.

Exécuter les ciblés sous poste lourd, lib complète29, -j8, target privé/plafond32G ; mesures prioritaires. Jeux natif puis code si nécessaire pour le raccord Landlock, sans services/GPU/modèles ni base réelle. La compilation minimale reste rouge sur des helpers préexistants distincts ; ne pas la rejouer inchangée pour annoncer un progrès. Relecture du diff et des preuves avant toute passe globale ou demande de tour master.
