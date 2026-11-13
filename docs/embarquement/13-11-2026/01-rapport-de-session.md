# Embarquement — reprise du 13 novembre 2026

## Travail en cours

Ticket porteur : [activation et résidence](../tickets/2026-11-12-gerer-activation-et-residence-des-modeles.md).
La tranche dense locale `maintenir` est implémentée et son union publication/ED
est qualifiée et livrée sur master par23ceed87b, après tour explicite.
Les trois contrôles de livraison sont verts : propre, aucun rebase,
origin/master égal à HEAD. La conception distante/éviction/baux distribués/crash
reste dans le même ticket ouvert, sans prétendre l'avoir réalisée.

Worktrees utilisés jusqu’au retrait gardé : `/home/lucied/git_workspaces/cx-activation-modeles`.
Branche de livraison `cx-activation-modeles-6` ; branches distantes précédentes
conservées. Les worktrees et le target sont jetables après livraison : s’ils
sont absents, repartir de master, ne pas restaurer leurs anciens caches. Source initiale84259404f puis union corrigée138268398, tous deux conservés.
Le seul ajout Rust entre les deux est le témoin déterministe08c/c44 d’ED. Ne pas modifier l'arbre distant pendant la passe.
Le target unique possédé est `~/.cache/rag3weaver-build/target-spans`, plafond
32 Gio ; magasin29a7bc52a complet, j8. Aucun code de l'arbre de Lucie modifié.

## Preuves déjà obtenues

- CPU commun : standalone17, connecteur5, chauffe1, aucun ignoré
  (`validation-jqwupheq`).
- Banc réel107m : acquisition partagée, annulation indépendante, réattachement,
  arrêt privé puis reprise ; deux départs, chauffe16/16 et vecteurs384 vrais
  (`validation-bvdguacm`). Rapport dans docs/banc/12-11-2026.
- Fournisseurs privés BGE et278 : tests2/2, observateur préattaché continu,
  résidence et calcul PCI04 observés pour les deux, aucun client PCI07 observé
  (`validation-tbwulyl4`). Données /proc indisponibles nommées inconnues.
- Union navigation : F5/5, lib2258 passés/65 ignorés structurels, NAV40/40
  (`validation-qdm6acnb`), fournisseurs2/2 et observation physique2/2
  (`validation-uu18g72q`), sparse réel1/1 (`validation-twt84wn7`).
- Union Pub/ED : recette `qualifier-union.py`, portée commune2400 s après
  acquisition lourd, CPU quatre rôles, fixture PG neuve, F5/lib/NAV et quatre
  suites catalogue3/1/18/3. Rouge `validation-hex7oup7` : F5/5 puis lib2272 passés/1 échec/66 ignorés.
  `a_progress_card_reaches_the_page_and_never_the_model` : deuxième carte
  show_progress absente ; pas de NAV/tx joués, restes[], bus et fixture nettoyés.
  ED a corrigé la fixture par08c379b11 (cherry-pickc44dfde1a),
  sans changement de production. Reprise138268398, `validation-5wmntc4y`
  VERT/code0/restes[]/bus nettoyé : F5/5, lib2273/0/66, NAV40/0/0,
  transactions3/1/18/3 sans échec ni ignoré. La barrière atteste un index réel
  actif pendant les deux cartes puis le cas terminé sans UI. Échéance
  877122,116265722 ≤ originale877122,611725539 ; aucun reset. Target final
  24407724032 octets. Fixture PG supprimée, unité inactive/code0.

Le code activation/serveur, ModelSource, fournisseur, consommateur et observateur
est identique entre branches3/4. L'orchestration demande de conserver leurs
preuves GPU, sans rejouer des modèles pour une rebase seule. Les lignes BAC
ajoutées par ED dans backend.rs sont conservées séparément du constructeur.

## Exploitation et limites

Services installés278/BGE conservés, aucun redémarrage dans ce lot. Audit des
résidents terminé par orchestration/optimiseur :107 standard et privé7882
arrêtés, gain8,637 Gio ; ticket d'audit fermé. Ne pas réactiver sans besoin.
Incident granite historique toujours ouvert, cause non établie ; attendre une
récidive lisible, ne pas transformer l'absence de journal en diagnostic.

Tous les refus, rouges et preuves sont conservés sous
`~/.cache/embarquement-notes/activation/`, hors target. Binaires immuables
mesurés345a402d0 et copies avant rebase3 gardés avec SHA256 ; ne pas les effacer
avec le cache. Les nettoyages sélectifs et leurs limites sont dans le ticket.

## Reprendre

1. Lire union-2.log et union-pub-ed/*.log ; confirmer preuve commune, restes[],
   nettoyage du bus/fixture et plafond target. Aucun renouvellement automatique.
2. Transmettre le bilan à l'orchestration ; ordre master Pub → ED → Embar.
   Attendre exactement « à toi », qualifier tout changement Rust véritable.
3. Les atterrissages documentaires d'un même parent code ne demandent pas un
   nouveau rejeu : comparer les arbres et préserver les empreintes des composants.
4. Appliquer docs/procedures/push-d-un-lot.md et la garde actuelle avant tout
   nettoyage : archives fidèles des administrations détruites, quiescence,
   garde VERT, jamais supprimer un sous-module partagé ni forcer un refus.


## Livraison et retrait

Livraison23ceed87b/tree3d8f853fa172e47a1fd41e27a167d950bf9e1032.
Premier rebase sous verrou abandonné sans push : il rejouait des bilans ED
anciens déjà livrés (quatre conflits MD). Reprise hors verrou sur branche6,
`rebase --onto origin/master c9e6b6e39` ; aucun conflit, diff produit vide
par rapport à a5977c930, puis dernier pas verrouillé et trois contrôles verts.
Revue finale Mémoire favorable après lecture des logs/proofs et équivalences,
bus physiquement absent, aucun nouveau défaut concret.

Sources138 attestées archivées hors worktree sous
`~/.cache/embarquement-notes/activation/sources-138268398.tar.gz`, SHA256
937ed60424230e5fd8e1d5b6a9e4809e42c1ead70b8f07be383ab1f8cf00e5e2.
Retrait préparé par poste : archives fidèles avant/après deinit des seules
administrations privées, préservation des données générées, zéro référence
active, garde actuelle et rapport neuf avant suppression sans force.
Les reçus définitifs sont `archives-fin-portable/retrait.json` et
`archives-fin-luciepc/retrait.json`, hors cible. Le reçu PC définitif est désormais `archives-fin-luciepc-e167-final/retrait.json`
(après correctif de garde intégré e167681867). W et target sont absents sur les
deux postes, bases PG0/sessions0. La garde PC finale est VERT/refus[], archive
fidèle et quiescence fraîche ; seul le drapeau structurel Git --force a été
autorisé par orchestration après ce vert. Aucun refus de garde contourné.
Le ticket de garde est clos avec ces preuves ; les anciens rouges sont gardés.

Services finaux inchangés :278 PID966256/NRestarts4, BGE PID3415586/NRestarts0.
Aucune demande de redéploiement dans ce lot. Le ticket global activation reste
ouvert pour les politiques non implémentées ; la régression de fixture est résolue.
