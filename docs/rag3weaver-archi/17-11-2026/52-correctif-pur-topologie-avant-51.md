# Correctif pur de topologie préparé après trois rouges conservés

La réserve Recherche/C7 sur cb290 a été consignée avant diff, les quatre témoins
prépubliés80c68 avant correction. Sous GO C7-ENTRY-TOPOLOGY-RED-BD0D-17nov60,
les trois négatifs sont vus **3FAIL «Refusal not raised»/0ERROR/skip/rc1**, une
seule invocation sans retry. Preuves2a715fd7b971a009fcf666c27ac50c5d87f344cf,
neuf bruts LOCAL+PC0écart, manifeste
3572a359673f698f96ce04a424721f6b4877c726cb88b450d2bbff8505ec22a2.
Le module était inchangé et les archives réhashées sur les deux hôtes AVANTfix.

Le seul changement du module pur :

- spawned refuse le PID enfant égal à runner, Common owner OU Common root ;
- common_root et runner de même PID doivent avoir le même start. L'égalité
  exacte reste permise pour un runner enfant direct de Common.

Aucun autre comportement/contrat/types/chronologie/CP/Common réel modifié.
Les51fonctions de témoins sont identiques à la prépublication80c68/bd0d ;
aucune assertion atténuée. Le positif root=runner exact n'a pas encore tourné.
Les47anciens et ce positif restent NON JOUÉS sur ce module corrigé. Les trois
négatifs ne sont pas rejoués ici. Il n'existe donc pas de51verts acquis.

[Pins proposés pour GO51 distinct](execution-entry-contract/pure-51-pins.json)
fixent module/test/stub/interpréteur, import/cwd/PYTHONPATH et une invocation
unittest-v-f test_execution_entry bornée TERM58/KILL2. Proposition sansGO,
executed=false. Demande RO source totale et pins avant décision C7/CPU.
Le rouge de la baseline nominale f74 et celui de topologie2a restent figés.

Ce module valide seulement des faits de modèle fournis par les propriétaires
de confiance. Aucun I/O, clock réelle, processus du protocole, signal, SDK,
CP/Common réel ou publication. Aucun backend, appelant, canon, target/service
ou native touché. L'avis CPU éventuel ne produira aucune admission physique.
Vérification actuelle : lecture source/diff et git diff --check seulement.
