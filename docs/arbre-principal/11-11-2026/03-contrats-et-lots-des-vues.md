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
- **Résolution.** Une couche cite sa parente et sa génération épinglée. Résoudre une occurrence
  descend récursivement cette pile ; une ancre qui avance ne change pas silencieusement une vue.
  Le tombstone d'un scope ou d'un fichier masque tous ses ancêtres concernés. Une vue inconnue ou
  ambiguë est refusée explicitement ; aucune recherche ne se replie sur une autre vue.
- **Résultats publics.** Les candidats peuvent venir de versions partagées, mais le résultat est
  toujours une occurrence visible : mêmes champs publics de Scope, fichier, position et liens de
  la couche résolue. Le target Scope du manifeste ne se remplace pas simplement par ScopeVersion.
  Les comportements after, fraîcheur et rendu conservent ce contrat.
- **Filtrage.** Vue ET domaine ET filtre utilisateur s'appliquent avant le top-k de chaque signal
  dense, creux et plein texte. L'admission porte sur l'occurrence visible, pas seulement sur
  (fichier, version). Un signal incapable de garantir ce masque refuse explicitement la requête ;
  il ne cherche jamais sans masque. Les arêtes doivent avoir leur source ET leur cible visibles.
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
~/.cache/rag3weaver-build/wt-deplace-codex ; target unique : target-A-plan ; lib épinglée :
rag3db-build-29a7bc52aaf11e6204f9beafaffb154105417e13-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF.

Journaux luciepc dans ~/.cache/rag3weaver-build/deplace-codex/ : rouge.log détecte un texte
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
