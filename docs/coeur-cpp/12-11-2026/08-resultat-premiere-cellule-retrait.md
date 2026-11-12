# Première cellule de retrait : trois paires terminées

Porteur cœur C++, [même ticket d’annulation](../tickets/2026-11-09-sous-vingt-ecrivains-un-pool-plein-laisse-une-annulation-inachevee.md).
Une cellule et six processus seulement : aucune matrice, aucun scénario DDL,
aucune recompilation après le premier arrêt de préparation.

## Résultat observé

N=65 536 entrées, R=4 096 retraits en 64 plages de 64 offsets, 256 partitions,
clés de préfixe long. Ordre : clé/plage, plage/clé, clé/plage ; processus neufs.
Durée ci-dessous : **somme des 64 appels de retrait**, hors préparation,
validation des survivants et réinsertion.

| Paire | Clés préconnues (ms) | Plages (ms) | Ratio plage/clé |
|---|---:|---:|---:|
| 1 | 0.295436 | 28.604142 | 96.82 |
| 2 | 0.296892 | 26.442622 | 89.06 |
| 3 | 0.303439 | 26.012750 | 85.73 |
| Médianes | 0.296892 | 26.442622 | 89.06 |

Le surcoût du mécanisme de scan par plage est mesuré ici sur des entrées
identiques. Un seul couple N/U ne mesure **pas** une courbe de croissance.
La boucle N×U est visible dans le code, pas un compteur de visites instrumenté.
Le mode clé retire des clés préconnues : référence basse sans relecture et
matérialisation des colonnes, **pas** le rollback complet du moteur de base.
Ce ratio ne démontre ni la cause du pool plein DDL ni un facteur de régression
du moteur complet. Aucun verdict global vert ou tour de push demandé.

## Égalités et mémoire

Les six processus vérifient 61 440 survivants et leurs offsets, absence des
4 096 retirés puis réinsertion réussie. Empreinte FNV1a64 d’entrée
`4679382657564329759`, survivants `7999543437703913983` :
identiques partout, comparés comme entiers exacts en Python. Les 384 plages
brutes sont exportées, et leurs volumes/temps recoupés avec les totaux.

Pool configuré512Mio, utilisé aux frontières7 647 744 octets dans tous les
appels des deux modes : **pas de plein de pool ici**. Ce constat ne prouve pas
l’absence d’allocation transitoire. RSS autodéclaré avant validation :
42 596 à42 736Kio.

## Bornes et libération

Poste mesure exclusif acquis avant les horloges. Cgroup réel1 073 741 824 octets/
swap0 vérifié, unité890s+arrêt2s sous global900s ; enfants57s+2s sous60s.
Aucun autre lourd dans la porte exclusive. Contrôleur0,558s, service0,652s,
CPU0,412s, pic cgroup49,6M selon journal. Code0, aucun dépassement/retry.
Unité arrêtée/MainPID0/poste0/4 relus, arbre et C7 prévenus de la libération.

## Binaire, commandes et bruts

Binaire distant conservé :
`~/.cache/rag3db-moteur-notes/annulation/cx-pilote-retrait-0511dece7/cx-mesure-retrait`,
25 610 008 octets, SHA256
`0c868053afea43df3cb2eb5fe8e77f597701b8ab4eb2a0c403ca854ef64cf704`.
SourceSHAfdc3e90f…, recetteSHA b4d424…, archive3030/manifest004911 et configuration
privée distincte déjà attestés : aucun objet de base/test/moteur recompilé ici.

Le refus ELF de préparation est conservé : chargeur système relu par C7 puis
requalification en lecture du même binaire, cinq composants exacts, sans élargissement
générique. Ancien contrôleur inchangé119db4c8… ; enveloppe force phase measure
uniquement et préserve le refus initial. Exactement six exécutions ensuite.

- [Preuve complète, SHA et argv](preuves/cellule-64k-64/preuve-execution.json).
- [Empreintes de l’export](preuves/cellule-64k-64/SHA256SUMS).
- Bruts : [1](preuves/cellule-64k-64/cellule-1.stdout), [2](preuves/cellule-64k-64/cellule-2.stdout),
  [3](preuves/cellule-64k-64/cellule-3.stdout), [4](preuves/cellule-64k-64/cellule-4.stdout),
  [5](preuves/cellule-64k-64/cellule-5.stdout), [6](preuves/cellule-64k-64/cellule-6.stdout).
- [Bornes réelles](preuves/cellule-64k-64/measure-limites.json),
  [journal unité](preuves/cellule-64k-64/unite-mesure.log),
  [requalification](preuves/cellule-64k-64/elf-requalification.json),
  [refus initial](preuves/cellule-64k-64/premier-echec.json).

## Suite à la revue seulement

Relire le coût du scan de toutes les partitions pour chaque plage avant de
considérer le lot prêt. Aucun nouveau build, mesure, matrice ou DDL automatique.
Le même ticket porte la décision sur le mécanisme et ses garanties ; le
différentiel DdlEveryMinute reste ouvert. Aucune lib distribuée modifiée.
