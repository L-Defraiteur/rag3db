# Armement effectif de la borne et revue pure des RAW ED

## Bac ciblé : correction groupée avant nouvelle préparation

APP7425375d66a64b0782672225c59516f048682b00 inchangée. Code pilote
291138a70b0aeb76aa2c5613fb317f921dd4af53. Ticket existant complété8505c89896
avant tout diff. Mémoire a réservé9f3 pour l'armement : l'écriture de launch.json
précédait la création de timeout780-k15. Un retard D ne devait pas s'ajouter à
la borne physique annoncée. Aucun des anciens couples n'a été lancé.

Contre-exemple du VRAI launch prépublié572b514bc6 : ROUGE1/0.002s sous GO
C7-ARCHI-ARMING-PURE-16nov60, première publication avant watchdog. Correctif
prépublié d8631c127f puis291138a70b (transport d'environnement du faux watchdog
corrigé avant premier jugement du fix) : douze fonctions CPU VERTES0.025s.
Aucun Cargo, processus natif, autorité, scan /proc réel ou service.

Le parent launch valide l'argv puis lance directement le vrai timeout780-k15
sur Q/qualify_green.py --arm et rend seulement son rc. Aucun log d'allocation,
précontrôle de cache ou clock de validation dans ce parent. Le worker arm,
déjà sous watchdog, observe une heure monotone puis fait tous les précontrôles,
fige l'identité de son vrai parent timeout et du lanceur, ouvre/redirige le log,
écrit/fsync l'allocation et execve le vrai poste. Le même PID devient le shell
poste ; la file, la portée, le final et sa publication restent sous ce timeout.
Le constructeur réel et la politique partagent le nouvel argv du watchdog.

`worker_observed_after_watchdog` est une observation APRÈS armement, pas l'instant
exact d'armement OS. Les soft/hard observés sont des majorants conservateurs :
le watchdog existait avant et peut arrêter plus tôt. Aucun nouveau temps de
780s n'est alloué après l'écriture. La portée reste600MAX et MINreste aprèsposte,
final120cap sous cette enveloppe ; correctif postfsync9f3 conservé.

Journaux hors W conservés :

- cx-doc-bac-arm-red-r1/tests.log :
  d42e0cb1b08f78bfcdcf24c5c67159e53279f31703435566d45880c0ce824f91 ;
- cx-doc-bac-arm-green-r1/tests.log :
  08c62b946b219d2fabc62933f865722f6435d7b41b79d1684a580fd5cb0c5a1e.

RO totales groupées demandées à Mémoire/Banc sur291. Ancien Qb9b et son paquet
restent figés ; nouvelle préparation/pin/copies après regroupement des réserves.
Pas de GO795/600 ni de lancement natif ; source742/T/archives inchangés.

## ED : revue pure du classificateur197b60eb

Revue exacte197b60ebc83ec0d4b6329dc977c85572a65a7312 dans le WT
rag3db-cx-ed-vertex-raw-classifier-r1 : lecture entière classify_raw.py,
AST3 sans import ni exécution des tests. Contrats par phase/noms/status/annotations
panic/comptes/ignored/filtered, deux filets puis suite, suffixe des huit jeux
après marqueur de précontrôle, ordre et résumés canoniques cohérents.
Aucun bloqueur statique repéré dans ce périmètre de classification figée.

Réhash réel des22fichiers du paquet et54RAW sur portable ET PC : zéro écart.
Manifest paquet000e53dd15a047a34b2fef8805740ef3f8b3fc243e1c304f415df010d5824934,
preparedc653aa17271c82892d92faa3ff43c52db49714583287aa530da50299e9fd189b,
RAW2754c91e37815873fbdfd4bbeff2800ca2b3f259f1f24397b2038c88a5a9ddde.

Avis favorable borné transmis ED/C7 : l'inventaire est observé APRÈS exécution,
figé AVANT classificateur, sans indépendance d'un ELF--list antérieur ; un absent
des RAW ne devient pas visible grâce à cet inventaire. Les72ignored de lib sont
consignés, jamais passés. RAW_CANONICAL_CLASSIFIED_ONLY conserve explicitement
historique contrôleurRED/scopeROUGE, sans qualification composite ni GO/reprise
de l'ancienne échéance. Aucun pytest/Cargo/ELF--list/natif/DB/service/cloud rejoué.
