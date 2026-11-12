# Une mémoire ne sait pas que ses gabarits sont en retard

- **État** : ouvert, le 7 novembre 2026. Forme tranchée par l'orchestration sur le mot de Lucie
  (« pense le plus cool et scalable ») ; pour la session mémoire (96), lot M8, après le pas 3 de M4.
- **Gravité** : blocage. Un backend ouvert sur une copie de `templates/` ne reçoit jamais les
  outils ajoutés au gabarit ensuite, et rien ne le dit.
- **Atteignable en service** : oui. La mémoire de Lucie (`~/.config/rag3weaver/memoire-longue/`)
  est une copie du 25 octobre 2026 au soir : `whois`, `select_sources`, `select_apports` et bientôt
  `archive` n'y sont pas.
- **Touche rag3weaver** : oui (le manifeste, l'hôte, la ligne de commande).
- **Famille** : produit

## Ce que c'est

Le dossier de configuration d'un backend est une copie de `templates/` faite à la main, sans
version : l'hôte ne peut pas dire qu'il est en retard sur les gabarits de son binaire, et la mise à
jour est une recopie à la main (la recette du 25 octobre, page 05 de la mémoire longue).

## Ce qu'il faut (la forme tranchée)

- Le manifeste d'un backend **épingle la version de ses gabarits** : `templates.version`, le commit
  ou l'empreinte du dossier `templates/` copié.
- À l'ouverture, l'hôte compare à `templates/` du binaire et dit, au journal et dans
  `daemon_status`, « gabarits en retard : +N outils (…), −M (…) », **sans rien changer**.
- `rag3weaver-backend templates upgrade <backend>` fait la montée : montre le diff des outils, ferme
  le backend par l'hôte, prend l'instantané, copie `templates/` à neuf (en gardant les champs
  propres au backend : `database`, `files_root`, `vector_extension`, `embeddings.address`), pose la
  nouvelle version, rouvre, vérifie (`index_state`, `verify_files`), et écrit la ligne dans la page
  du jour. Le même geste pour cent mémoires ; un geste par mémoire, rien d'implicite.

## Le témoin

Une config épinglée sur une version plus vieille que le gabarit : l'ouverture dit le retard et sert
les outils d'avant ; `templates upgrade` : les outils neufs sont servis, la base est intacte champ à
champ, l'instantané existe, les champs propres sont gardés.

## Pour le fermer

Le lot M8 sur master avec son témoin ; puis la mémoire de Lucie montée une fois par la commande, au
mot de l'orchestration, quand `archive` est sur master.

## Reprise et dépendances au 11 novembre

Porteur : session mémoire. La [page de conception M8](../../../extension/rag3weaver/docs/09-novembre-2026-memoire-longue/02-m8-la-montee-des-gabarits.md) attend la relecture d’everything declarative ; aucun lot de code n’est ouvert par cette régularisation. Le rapport du 11 novembre décrit aussi une remise documentaire de l’ancien worktree `wt-mf-96` à relire avant de la reprendre : ne pas l’appliquer aveuglément ni confondre cet état historique avec un travail déjà livré.

Le correctif archive `41526b29a` ne monte pas la configuration réelle de Lucie. [L’archivage à blanc restant](2026-11-11-terminer-l-archivage-a-blanc-des-documents-sur-copie.md) est séparé ; [le raccord automatique M9](2026-11-11-raccorder-l-archiviste-automatique-au-chat.md) ne l’active pas non plus. Après relecture, préciser adoption/fusion/migration et leurs refus, poser les témoins puis la passe avant de demander un tour de code. La montée réelle garde sa décision explicite et son instantané.

## Relecture du 12 novembre 2026

**État : ouvert ; porteur mémoire.** La [revue ED sourcée](../12-11-2026/01-relecture-m8-adoption-fusion-et-reprise.md)
distingue constats, recommandations et choix non tranchés. Aucun lot de code M8 n'est ouvert.
La conception courante est révisée : identité Gmail, distinction du suivi vivant et de l'épingle,
base historique vérifiée, fusion avec conflits et maintenance/reprise par contrats communs.

À trancher avant un lot : politique vivant/épinglé, conservation des bases, qualification des
transitions et exclusion/reprise multi-ressources. Première tranche proposée : plan à blanc,
fusion pure et validation en recouvrement ; aucun apply ni livre écrit. Le diagnostic 1b n'est
pas une application de migration livrée. Les critères de fermeture restent ceux du ticket :
implémentation et témoins livrés, puis montée réelle distinctement autorisée et prouvée.
Aucune base réelle modifiée, aucun test relancé ; Gemini rouge inchangé.

## Complément mémoire du 12 novembre — effets de la validation à cadrer

La revue ED est maintenant intégrée sous `aa741a4c3bbd48a3194a802cfab1c8612cdd8a57` dans [la note canonique](../12-11-2026/01-relecture-m8-adoption-fusion-et-reprise.md). Mémoire en reprend le suivi ; aucun lot de code n’est ouvert. La référence « précontrôle 1b » est `85ef4921128b445e74766c1e628809b206957485`, pas un hash préfixé par le nom du lot.

**Constat par lecture, sans exécution** : à `da69b842c3` et `7004350fb`, `PreparedBackend::load_avec` appelle `builtin_templates::resolve` si un manifeste contient `builtin:` ; `resolve` appelle `root`, qui peut créer et extraire les gabarits en cache lorsqu’ils manquent. `Recouvrement` évite de réécrire les déclarations candidates mais ne prouve pas, à lui seul, une validation sans aucune écriture. Les fonctions source et la limite sont reliées dans [le rapport de reprise](../12-11-2026/01-rapport-de-session.md).

**Reste à faire dans M8**, avant de promettre le premier plan pur : préciser la frontière des effets et le mode de résolution des gabarits, puis prévoir un témoin avec cache builtin vide, en plus de la non-mutation du manifeste/livre/base. Ne pas dépendre d’un cache déjà chaud pour conclure à l’absence d’effets ; ne pas élargir implicitement le contrat pour obtenir un vert. Ce point complète la revue existante, sans nouveau ticket de migration ni nouveau moteur M8. La maintenance, la reprise et toute adoption/application réelle gardent leurs contrats communs et leurs preuves distincts.

## Première tranche proposée : frontière pure et adaptateur de sources séparés

Le [contrat documentaire de la première tranche](../12-11-2026/01-relecture-m8-adoption-fusion-et-reprise.md) précise les entrées base/local/cible et leurs contenus, la fusion/validation, les sorties et les refus. Le cœur ne lit pas le disque par repli, n’extrait pas les builtin et n’écrit ni cache, manifeste, livre, base, objets ni journal. Résolution depuis les contenus fournis/embarqués ; source requise manquante = refus nommé.

Une éventuelle préparation des sources par le CLI est une frontière distincte avec effets explicités ; elle ne doit pas déplacer une extraction hors champ pour faire paraître pur le plan. `Recouvrement`/`load_avec` actuels ne constituent pas encore la preuve de ce contrat. Témoins à prévoir : cache froid/vide ou inaccessible, dépendance manquante malgré un fichier présent seulement sur disque, résultats indépendants de fichiers étrangers, aucun effet même sur refus. Aucun de ces témoins n’a été exécuté.

La proposition n’effectue ni adoption persistée ni apply, ne prouve pas la compatibilité des données réelles et n’autorise aucune activation. Les choix vivant/épinglé, stockage des bases et maintenance/récupération restent explicites. Aucun lot de code n’est ouvert par ce cadrage ; M8 reste ouvert.

## Prise en charge code du 12 novembre — première tranche autorisée

Autorisation : [répartition des suites](../../tickets/closed/2026-11-12-paralleliser-les-suites-autorisees-sans-chevauchement.md), master `6e5f9458c`. Porteur mémoire ; branche `cx-memoire-m8-plan`, worktree `cx-wt-memoire-m8-plan`, depuis ce master. Le lot reste un plan à blanc, sans apply, adoption persistée, changement de politique ou activation réelle.

API à borner : instantanés de contenus base/local/cible, provenance et empreintes ; fusion pure JSON et fichiers avec absent distinct de null, conflits structurés ; candidat validé par les validateurs communs à partir des seules sources fournies/embarquées ; diff d’outils déterministe sans identité de renommage déduite ni application autorisée. Une source requise manque : refus nommé. Aucun CLI d’acquisition persistante dans ce lot.

Fichiers proposés possédés : nouveaux `src/fusion.rs` et `src/template_plan.rs` et leurs témoins ; déclarations de modules dans `src/lib.rs`. Primitives communes nécessaires à coordonner avant modification : `src/lecteur.rs` (lecteur fermé sur contenus, identité de chemin sans canonicalisation disque), `src/builtin_templates.rs` (résolution sans extraction), `src/backend.rs` (point de chargement/validation commun ; ED possède la correction des accords dans ce même fichier). Le traitement des sous-graphes builtin et des contrôles d’environnement doit être audité ; signaler toute extension de ce périmètre avant modification. Ne pas créer un second validateur M8.

Premières preuves visées : témoin rouge du chargement commun encore dépendant du disque/cache, puis fusion/conflits et lecteur fermé ; cas cache vide ou inaccessible, fichier présent seulement sur disque, sources ambiguës/manquantes, candidat invalide et refus des changements non pris en charge. Aucun de ces nouveaux témoins n’est encore joué à cette prise en charge. Les résultats ciblés et le diff seront relus avant toute passe globale ou demande de tour master.

Builds ciblés à luciepc : target privé `~/.cache/rag3weaver-build/cx-target-memoire-m8-plan`, -j8, plafond32G, poste lourd et lib29 épinglée ; priorité aux mesures convenues. Aucun service/GPU/modèle/base réelle nouveau. Le nettoyage final devra passer la garde commune avec archive administrative externe.

Premier ciblé arrêté avant exécution : la compilation sans natif échoue dans un témoin existant de liens, suivi par [le ticket canonique tty](../../tty/tickets/2026-11-07-la-lib-rag3weaver-ne-se-batit-pas-sans-rag3db-native.md). Ce n’est pas le rouge comportemental M8. Reprise avec rag3db-native, même cible mémoire/cache, sans ouvrir de base ; journaux séparés. ED et orchestration ont confirmé la possession M8 de backend.rs/lecteur.rs/builtin_templates.rs et, si nécessaire, foreach_node.rs ; la correction des accords ED reste dans commande.rs/run_nodes.rs.

Le doublon de signalement a été fermé comme remplacé, sans fermer le ticket tty. La qualification cfg est isolée sous `80faa4838` dans `cx-memoire-cfg-temoin-natif` ; source du ciblé M8 natif en file toujours inchangée à `59b3f0ff7`.

API/fichiers et limites du code en branche publiés dans [le suivi de tranche](../12-11-2026/02-plan-m8-api-et-preuves.md). Rouge de cache réellement observé sur59b3f0ff7 :0passé/1échoué, processus neuf, extraction à froid pendant un refus ; log luciepc cx-memoire-m8-rouge-natif.log. Aucun vert du nouveau code n’est encore déclaré. Contrat conservateur : fichiers atomiques et changements de schéma/format non qualifiés refusés ; JSON Schema externe non pris en charge par le résolveur fermé refuse, sans accès disque/réseau. Les effets d’acquisition CLI et tout apply/persistant restent hors lot.

Premier ciblé source95dc16908 : fusion4/0/0 ; plan5 réussites/8échecs/0ignoré, arrêt au premier filtre rouge. Fixtures obligatoires à corriger, aucun vert global. Résultats et corrections suivis dans [le suivi de tranche](../12-11-2026/02-plan-m8-api-et-preuves.md). Avant globale : reposer sur master3b2eab7ec sous branche neuve et relire le diff, selon orchestration.

Ciblés verts de la tranche sur caef19714b6edcef70961d6332b09528db4f0a91 :35/0/0 (fusion5, plan15, lecteur5, builtin9, foreach1), natif+code/lib29/poste32G/j8. Preuves03 et limites dans le suivi de tranche. M8 reste ouvert ; revue avant globale, aucune application réelle autorisée.

Revue orchestration après ciblés : la garde omet storage (admission réelle exécutée à l’ouverture seulement), vector_extension (ressource physique), seed (semis de données à l’ouverture). Porteur mémoire, même tranche : témoin de refus des ajouts/retraits et changements format/recette storage, changement d’extension et semis ; puis garde conservatrice sans admission DB ni moteur de compatibilité nouveau. Le vert ciblé précédent ne couvre pas ces transitions.

Complément storage/vector_extension/seed corrigé757e3b1a9 après rouge103fded7e : ciblés36/0/0, dont les6transitions non qualifiées refusées. Détail et preuves04 dans la note02 ; aucune globale ni application réelle, ticket toujours ouvert.

Orchestration autorise la globale après revue du complément757 : rebase sur masteraf2ef6846, nouvelle branche cx-memoire-m8-plan-3, range-diff11commits identiques. Passe prévue : features5/5, checklibminimal, liblarge, navigateur39/40, puis8suites100tests (demon_hote13, lectures24, mcp_cycle_de_vie11, mcp_stdio4, fichiers_entite3, fichier_compose4, migration_preflight5, postgres36). Driver externe borné3600s après poste lourd, mémoire32G/j8/targetM8unique32G/lib29 ; PGprivée neuve sur serveur existant, React existant, TMPcourt disque, santé réelle7879 avant e2e. Premier rouge arrête ; sources/binaire/assets220/compteurs/quiescence à attester. Aucun modèle/serviceGPU nouveau ni Gemini historique ni application réelle. Verdict relu et tour explicite requis avant master.

Globale1 sourcee9e0f461f interrompue volontairement pendant compilation lib, après features5/5 et checklibminimal verts, aucun testlib encore exécuté : driver omettait RAG3WEAVER_JOURNAUX_DIR, donc STATE_DIR seul ne fermait pas le cache des commandes. Signal ED sur contrainte commandes/index des fixtures (ticket recherche transmis35c23c2a6). Correction du driver externe avant reprise : journaux privés finissant par commandes, builtin cache privé, portée liée à l’unité par BindsTo pour arrêter aussi les enfants si unité stoppée. Scope1 arrêtée explicitement/inactive, verdictROUGE préservé ; PGprivée1 conservée. Pas de relance inchangée ; tentative2 neuve, mêmes bornes et source publiée.
