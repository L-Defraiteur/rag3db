# Retrait des clés — protocole borné, avant réservation d’un créneau

Porteur : cœur C++. Travail et clôture restent dans le [ticket d’annulation](../tickets/2026-11-09-sous-vingt-ecrivains-un-pool-plein-laisse-une-annulation-inachevee.md).
Ce document prépare une mesure ; aucune mesure ni compilation déclenchée par sa rédaction.

## Question et limites

Le correctif `0511dece7` parcourt 256 partitions locales pour chaque plage undo ;
la base `65486c02d` relit les clés des groupes sélectionnés, puis les retire.
On veut isoler la dépendance au nombre d’entrées locales et au nombre de plages,
sans modifier les volumes en fonction du temps ni reproduire le scénario DDL minute.

Les deux exécutions DDL déjà terminées ne sont pas appariées. Le pic du pool diverge
avant le troisième DDL ; leurs temps et volumes ne prouvent ni régression ni innocuité.
Conserver intégralement `base.log`, `isole-*.log`, les XML et les binaires sauvegardés.

## Inventaire et réemploi avant toute compilation

À luciepc, les sources et binaires courants de `rag3db-colonnes` correspondent à
la base ; les binaires du correctif sont sauvegardés dans
`~/.cache/rag3db-moteur-notes/annulation/binaires-0511dece7/`.
Les huit recettes multi-COPY ont **déjà** passé des deux côtés ; leurs XML suffisent
comme contrôles fonctionnels existants. Ne pas les rejouer pour remplir un tableau.

Les exécutables existants ont des recettes fixes, sans paramètres N × U. Ils ne
permettent pas de produire cette matrice. Le binaire de test du correctif n’est pas
une bibliothèque réutilisable par un nouveau pilote. Ne pas annoncer un essai
base/fix apparié en changeant seulement les variables d’environnement.

Lecture préalable, sans exécuter de test :

```bash
ssh -T -o BatchMode=yes lucied@luciepc bash -s <<'READ'
set -eu
cd "$HOME/git_workspaces/rag3db-colonnes"
git diff --exit-code 65486c02d -- src test extension/vector extension/geo
sha256sum build/moteur/src/librag3db.a
cat "$HOME/.cache/rag3db-moteur-notes/annulation/binaires-0511dece7/SHA256SUMS"
"$HOME/.cache/rag3weaver-build/poste" --qui
READ
```

## Étape A — hypothèse de coût du retrait dans l’index

Préparer un petit pilote externe au worktree de bâti, puis faire relire son source
avant compilation. Aucun chemin du moteur ou de la lib distribuée n’est modifié.
Le pilote utilisera le `InMemHashIndex` en mémoire, dans 256 partitions comme le
moteur. Les clés et leurs offsets existent déjà avant le chronométrage.

Deux modes dans le **même** exécutable, mêmes entrées et ordre :

- `key-discard` : retrait des clés déjà connues via `deleteKey`, comme le retrait
  d’index de la base, sans le coût de lecture/matérialisation des colonnes ;
- `range-discard` : `discardRange` exact de `0511dece7`, par plages d’offsets.

Le premier mode est une référence basse du retrait de la base, **pas** son rollback
complet. L’intitulé des résultats doit le dire. Aucune inférence directe sur les
2 048 Mio du scénario DDL à partir de cette seule micro-mesure.

L’archive statique de base est disponible : 63 Mio. Les méthodes sous-jacentes
utilisées par `InMemHashIndex` et sa disposition sont identiques sur les deux
sources ; seul le retrait par plage est ajouté en ligne dans son en-tête.
Avant le moindre lien, confirmer le diff de cet en-tête et de ses dépendances.
Si cette identité n’est pas établie, arrêter : pas de bricolage d’ABI.
Sinon, compiler uniquement le pilote avec cet en-tête exact en surcouche, puis
lier aux archives déjà bâties de la base (reprendre les options du lien du test
stockage via `ninja -t commands`, lecture seule). Pas de rebuild entier du moteur.

### Matrice et entrée constante

- N total d’entrées : 16 384, 65 536, 262 144, réparties dans les 256 partitions.
- R = 4 096 offsets à retirer, identiques pour tous les U d’un N donné.
- U plages : 1, 8, 64, 256 ; chacune retire R/U offsets, sans recouvrement.
- Clés longues de 128 caractères de préfixe + identifiant décimal ; générateur,
  graine, ordre d’insertion et répartition figés, empreinte du jeu exportée.
- Trois paires par cellule ; ordre alterné key/range, range/key, key/range.
- Un processus neuf par mode et répétition ; aucune croissance de N pendant la mesure.

Cela fait 12 cellules et 72 processus au maximum. Aucun échauffement chronométré
partagé entre modes ; construction, réserve et génération sont hors chronométrage.
Faire d’abord la seule cellule N=65 536, U=64 : si l’hypothèse ne se mesure pas ou
si le pilote ne vérifie pas ses invariants, arrêter avant la matrice complète.

### Mesures et contrôles

Une ligne JSON par plage et un bilan : N, R, U, graine/empreinte, mode, répétition,
entrées visitées (borne analytique N × U pour le scan, sans compteur inventé),
entrées avant/après, nombre effectif de retraits, latence de chaque appel et totale,
CPU utilisateur/système, RSS maximal, mémoire du pool avant/après et maximum observé.
Ne pas présenter une mémoire avant/après stable comme preuve d’absence d’allocation
transitoire ; une telle preuve exige un compteur de demandes, ou une instrumentation
séparée. Éviter le sampler concurrent pour les appels très courts.

Exiger exactement R retraits, N−R survivants, même empreinte finale des couples
clé/offset, recherche exacte des survivants et absence des retirés. La réinsertion
hors mesure doit restaurer N entrées et leur visibilité. Une erreur arrête la cellule.

L’unité du compteur U est ici **plages de retrait**, pas enregistrements undo du
moteur. Ne pas confondre les deux dans les tableaux.

### Plafonds et commande à préparer

Après relecture du pilote, pour le créneau : une seule mesure à la fois sous
`poste mesure`, pool 512 Mio, scratch sous `~/.cache/rag3db-moteur-notes/annulation/`,
aucun `/tmp` volumineux, aucun service GPU/7970, aucun changement des plafonds du poste.
Budget RSS 1 Gio observé et arrêt du seul groupe de la mesure si dépassé ; maximum
60 s par processus, 15 min pour le plan complet. Un dépassement est une donnée
censurée, pas un zéro ni une borne assouplie. Si le pilote n’a pas encore la gestion
de son groupe, ne pas le lancer par un simple `timeout` qui laisserait des enfants.

Commande contractuelle du pilote **à créer et relire**, pas un outil déjà disponible :

```text
cx-mesure-retrait --plan plan-index-undos.json
  --pool-mib 512 --rss-max-mib 1024 --cell-timeout-s 60 --total-timeout-s 900
  --output <dossier-neuf>/resultats.jsonl
```

Son lanceur doit envelopper cette commande dans `poste mesure`, enregistrer
source/compilation/hashes/options exacts, et sortir sans lancer la matrice si
l’empreinte des entrées diffère entre deux modes.

## Étape B — annulation complète, seulement si nécessaire après A

Si A montre une croissance matérielle du retrait par plage, l’hypothèse est établie
pour cette primitive ; cela motive une décision sur son coût, pas une refonte automatique.
Si A ne l’explique pas, ne pas répéter DdlEveryMinute : préparer un pilote moteur
à volumes fixes, mêmes N lignes validées et R lignes non validées, U COPY explicites,
auto-checkpoint coupé, un fil, aucune écriture de fond, aucun DDL vectoriel.

Ce pilote doit compter les véritables INSERT_INFO produits (un COPY n’est pas
nécessairement un undo), mesurer préparation et ROLLBACK séparément, relever pool,
RSS et demandes mémoire, puis vérifier lignes validées, clés annulées réutilisables
et réouverture. Il doit être compilé avec les deux versions exactes du moteur ;
l’archive du correctif n’a pas été sauvegardée. **Un rebuild assorti serait donc
nécessaire**, à proposer explicitement avec son créneau après A, sans toucher aux
libs distribuées. Aucun mélange de binaire et d’extension de générations différentes.

## Critères de conclusion

1. Entrées ou volumes différents : paire invalide, aucune attribution.
2. Sur entrées identiques, surcoût croissant avec U et N dans range-discard,
   indépendant de la préparation : preuve du coût de scan de cette primitive.
3. Pas de divergence A mais divergence du rollback complet B : chercher ailleurs
   dans l’annulation et les allocations ; ne pas l’attribuer à discardRange.
4. Coût A établi mais pression B identique : ne pas confondre latence et épuisement du pool.
5. Un seul essai hors DDL ne clôt pas le différentiel DdlEveryMinute. Conserver
   l’inconnue tant que la chaîne jusqu’à la pression n’est pas démontrée.

À partager à l’orchestration pour le créneau après relecture du pilote, avant mesure
ou rebuild. Pas de demande de tour de push C++, pas de publication de lib.
