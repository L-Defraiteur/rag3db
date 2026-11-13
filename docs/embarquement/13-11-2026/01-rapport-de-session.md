# Embarquement — reprise du 13 novembre 2026

## Travail en cours

Ticket porteur : [activation et résidence](../tickets/2026-11-12-gerer-activation-et-residence-des-modeles.md).
La tranche dense locale `maintenir` est implémentée, qualifiée sur la navigation
8f604 et empilée sur publication31db55abb puis EDc9e6b6e39. Elle n'est pas
encore livrée sur master. La conception distante/éviction/baux distribués/crash
reste dans le même ticket ouvert, sans prétendre l'avoir réalisée.

Worktrees portable et luciepc : `/home/lucied/git_workspaces/cx-activation-modeles`.
Branches : `cx-activation-modeles-4` (portable) et `cx-activation-modeles-pc-4`
(luciepc). Source en qualification CPU :84259404f ; les commits suivants sont
uniquement documentaires. Ne pas modifier l'arbre distant pendant la passe.
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
  suites catalogue3/1/18/3. En cours ; ne pas annoncer ce verdict avant preuve.

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
