# Vues par worktree et pyramide de couches : contrats et lots

11 novembre 2026. Cap confirmé par Lucie via orchestration : un index ancre par dépôt,
des différences au grain du scope et un héritage de couches, sans copie de base par tâche.
Ce complément précise la conception du 8 novembre après les relectures ED et recherche.
Il ne décrit pas une implémentation déjà livrée.

## Contrats issus des relectures

- **Empreinte et stockage.** L'identité de contenu couvre l'entrée réellement chunkée, y compris
  titre, signature et docstring s'ils y participent, ainsi que les versions de normalisation et de
  découpage. Les vecteurs restent distingués par modèle et dimension. Deux positions ou deux
  fichiers peuvent réutiliser un contenu ; cela ne fusionne pas leurs occurrences ni leurs arêtes.
  Au départ, conserver les octets du contenu : toute normalisation future exige un remappage prouvé
  des highlights et coordonnées vers le texte de chaque occurrence.
- **Résolution.** Une couche cite sa parente et sa génération épinglée. Résoudre une occurrence
  descend récursivement cette pile ; une ancre qui avance ne change pas silencieusement une vue.
  Le tombstone d'un scope ou d'un fichier masque tous ses ancêtres concernés. Une vue inconnue ou
  ambiguë est refusée explicitement ; aucune recherche ne se replie sur une autre vue.
- **Résultats publics.** Les candidats peuvent venir de versions partagées, mais le résultat est
  toujours une occurrence visible : mêmes champs publics de Scope, fichier, position et liens de
  la couche résolue. Le target Scope du manifeste ne se remplace pas simplement par ScopeVersion.
  Les comportements after, fraîcheur et rendu conservent ce contrat.
- **Filtrage.** Le masque de vue et les filtres usuels s'appliquent avant le top-k de chaque signal
  dense, creux et plein texte. L'admission porte sur l'occurrence visible, pas seulement sur
  (fichier, version). Un signal incapable de garantir ce masque refuse explicitement la requête ;
  il ne cherche jamais sans masque. La précédence domaine/utilisateur existante reste une décision
  séparée : ce contrat exige le masque de visibilité en plus du filtre effectif, sans réécrire cette
  politique. Un filtre utilisateur visant un scope tombstoné ne le rend pas visible. Les arêtes doivent avoir leur source ET leur cible visibles.
- **Cohérence.** Masque, candidats, enrichissement et provenance proviennent du même instantané L6.
  Un lecteur ne traverse pas deux générations pendant la résolution.
- **Outils.** view est un paramètre typé commun aux graphes d'outils ; provenance est un booléen
  optionnel séparé. Sans provenance, le rendu ordinaire reste stable. Une branche sans worktree
  se lit par blob/composite ; une écriture exige un worktree résolu. Le disque seul ne représente
  pas toutes les vues.
- **Migration.** Format déclaré, tables ajoutées et rattachements explicites ; index confirm ne
  déclenche pas une migration silencieuse. Une base ancienne hors contrat est refusée clairement.
  La déduplication peut diminuer les comptes de chunks/vecteurs : prouver la couverture de toutes
  les occurrences, la réutilisation des contenus/vecteurs, zéro appel à l'embarqueur et des recherches
  équivalentes, plutôt qu'une égalité brute des comptes. Les relations existantes sont conservées
  et les deux sens contrôlés avant/après sur copies privées.

## Lots proposés et preuves attendues

| lot | changement | preuves avant atterrissage |
|---|---|---|
| 0 | garde-fou du scope déplacé, seul, sans changement produit | mutation désactivant la réutilisation : réembarquement détecté ; restauration : test vert ; passe et e2e code/code_sync |
| 1a | version de contenu sur ancre unique ; occurrences et arêtes distinctes | deux fichiers identiques partagent contenu/chunks/vecteurs ; titre/docstring/chunking distincts ne collisionnent pas ; modèles distincts restent distincts ; position déplacée sans nouvel embarquement |
| 1b | migration déclarée de l'ancre | copie d'ancien format, couverture occurrences, zéro embarquement, recherches et deux sens avant/après ; ancien format non migré refusé explicitement |
| 2a | couches et résolution pure, générations épinglées, tombstones | f de parente, g de fille, h d'ancre ; suppression héritée ; ancre avancée sans effet implicite ; vue inconnue refusée |
| 2b | filtres d'index et outils dans L6 | top-k saturé de candidats invisibles sans fuite ni perte de résultats visibles ; version partagée entre occurrences ; filtres combinés ; arêtes avec cible invisible exclues ; provenance cohérente |
| 3a | ouverture et suivi worktree sans copie de base | trois fichiers changés : seules différences indexées, aucune copie ni deuxième base ; seuls contenus neufs embarqués ; branche sans worktree lisible |
| 3b | fusion/rejet et rétention | fusion dans parente sans réembarquement ; version partagée ou citée conservée ; lecteurs épinglés protégés ; orphelins retirés sans fuite entre vues |

Chaque lot produit commence par ses témoins rouges, puis passe complète avec suites du changement.
Le découpage 1a/1b permet de garder la migration observable et réversible sur copies avant toute base
partagée. Le contrat ne fixe pas encore une politique de compaction ou un cache de masques : ces
choix attendent les chiffres ci-dessous.

## Mesures M3–M5 : prérequis, pas de résultat

L'optimiseur a vérifié le 11 novembre à 17:37:28 les instantanés avant-43 sur luciepc :
code-principal, fichier DB de 1 044 275 200 octets (dossier 1,6 Go) ; code-lourd,
fichier DB de 1 246 605 312 octets (dossier 1,9 Go). /home est en btrfs : copies reflink possibles.
Ces instantanés sont au format 42 ; la lib épinglée 29a7bc52a est au format 43 : vérifier
l'ouverture et la migration uniquement sur de nouvelles copies privées, jamais sur les originaux.

Le témoin b826a1147 de arbre-principal/mesure-vues doit être repris sur master. M3 mesure coût du
filtre par appel ; M4 rappel sous filtre minuscule ; M5 impact des lignes mortes, en écriture sur
une autre copie privée. Ne lancer qu'avec file luciepc vide et mot de l'orchestration. À la lecture
des prérequis, 4/4 lourds étaient occupés et mémoire attendait. Le service 7882 reste réservé au
banc A/B codeparsers. Aucune mesure de cette reprise n'est exécutée ; aucun chiffre de performance
nouveau n'est revendiqué.

## État du garde-fou

Lot 30b5b146e, sauvegardé sur arbre-principal/deplace-sans-reembarquer-2 après fetch et rebase
(retard initial : 192 commits, sous le seuil 200). Worktree de vérification sur luciepc :
ancien worktree de validation distant (retiré) ; target alors utilisé : target-A-plan ; lib épinglée :
rag3db-build-29a7bc52aaf11e6204f9beafaffb154105417e13-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF.

Journaux luciepc dans ~/.cache/rag3weaver-build/cx-deplace-preuves/ : rouge.log détecte un texte
réembarqué sur un, test échoué ; vert.log après restauration rend 1 passed, 0 failed et le diff
produit est vide. La mutation temporaire neutralise le filtrage dense par marqueur et la
réutilisation KnownVectors ; elle n'entre dans aucun commit. check_features : VERT 5/5.
La lib entière a rendu 2 095 réussis, 0 échec, 58 ignorés ; le navigateur et les e2e
restent à confirmer. La passe complète reste en cours ; aucun verdict global ni tour de push n'est acquis. Un premier
lancement a été interrompu pour imposer CARGO_BUILD_JOBS=8 au contrôle des features ; il ne vaut
pas résultat de passe.

## Premier lot produit minimal proposé

Le lot 1a introduit le contrat de version de contenu sur l'ancre, sans ouverture de couche de tâche
ni migration des bases partagées. Ses témoins minimaux sont : même entrée réellement chunkée dans
deux occurrences → un contenu partagé ; titre/docstring ou version de découpage différents →
versions distinctes ; modèles d'embarquement différents → vecteurs distincts ; déplacement de
position → contenu réutilisé ; projection vers deux occurrences → fichiers/lignes propres à
chacune. La migration déclarée et ses preuves arrivent dans 1b avant toute bascule de base.
Les lots suivants emploieront branches et worktrees préfixés cx-, selon la convention du 11 novembre.

## Validation terminée et rouge du lot suivant

Le garde-fou est livré sur master1904e04e4 : features5/5 ; lib2095/0/58 ignorés ;
navigateur33/0/0 ; e2e_code27/0/0 ; e2e_code_sync17/0/0. Trois contrôles après push lus propres,
worktrees du garde-fou et target-A-plan nettoyés. Cette mise à jour remplace l'état « en cours »
de la section historique ci-dessus.

Les deux témoins du lot1a sont sauvegardés sous dd5f30465 (cx-arbre-scope-version) et joués :
1 passé, 1 échoué ; le défaut observé est deux embarquements d'un contenu identique pour deux
occurrences dont les coordonnées sont pourtant correctes. L'implémentation produit1a n'est pas
livrée. Le contrat reste déclaratif : composer les mécanismes existants et identifier la primitive
manquante de partage/projection, plutôt que coder une seconde chaîne de recherche réservée à Scope.

## Projection livrée séparément du stockage

Master6c4b5ad0c livre la projection dense/creuse d'un morceau vers toutes ses occurrences admises,
avec leurs champs, positions, attribution, meilleurs scores et ordre stable. La passe complète v3
et le rejeu court après ED sont dans le rapport 01 : 87 tests e2e passés dans quatre suites, aucune
suite sautée. Les nouveautés et le README expliquent cette capacité. Les versions de contenu et
les vues en couches restent non livrées ; le ticket du double embarquement est ouvert.

La préparation 1a est reprise sur cx-arbre-scope-version-3 ; lire
[04-identite-des-derivees-et-projection.md](04-identite-des-derivees-et-projection.md). La relation
vers la racine existe déjà pour les dérivées ; l'identité par contenu, le suivi par lien/racine et
la projection déclarée doivent être composés et prouvés. Aucune migration de base vivante lancée.


## Clarification Lucie — vues/checkpoints futurs, non implémentés ici

La vue courante est choisie automatiquement par le contexte de consultation ;
la provenance gagnante est PAR OCCURRENCE (scope hérité peut citer Pparent,
autre scope remplacé peut citer Pworktree). Consulter une version ancienne
est explicite : si déjà indexée, aucune ingestion ; sinon matérialiser
l’état visé sans indexer tous les commits intermédiaires.

Pour lots3a/3b, étudier des checkpoints logiques COMPLETS de visibilité,
occurrences, arêtes, provenance et références contenus/index partagés, puis
deltas ; ce n’est PAS un clone d’index. Un checkpoint qui ne porte qu’un
delta dépend encore de ses parents : ne pas jeter ces dépendances.
Chargement RAM à la demande et politique de rétention historique sont
distincts. Aucun coût constant, code de checkpoint livré ou politique GC
implémentée revendiqué. Clarification future du contrat, aucun changement
APP236e, de la qualification2a ou de l’autorisation de sonde.

## État après livraison2a et préparation2b

Masterd17 livre uniquement le résolveur pur2a (qualification composite rapport25, finressources27). Les sections historiques précédentes conservent leurs états à leur date. La [préparation2b](../13-11-2026/28-plan-visibilite-couches-2b.md) identifie admissioncommune et prérequis chargeurphysique/snapshots DB+handles ; aucun filtre de couche branché aux index/L6/outils par ce plan. Tests contractuels spécifiés, non écrits/non joués. Les checkpoints et le courant automatique restent des contrats futurs sans réalisation présumée.
