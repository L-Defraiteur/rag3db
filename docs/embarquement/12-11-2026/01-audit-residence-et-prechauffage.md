# Résidence des modèles : audit et ajustements du 12 novembre

Demande de Lucie : employer card2 et les modèles utiles sans accumuler les
résidents inutilisés. [Ticket d'audit clos](../../tickets/closed/2026-11-12-revoir-les-services-d-embarquement-residents.md).
Opérations par optimiseur, analyse par embarquement, recoupement avec mémoire,
arbre et archi ; consolidation par orchestration. Aucun modèle ou index supprimé.

## Décisions appliquées

Résidences par processus/PCI relevées vers 18:43 avant les arrêts ; chiffres
arrondis, distincts des allocations totales et de la VRAM globale de la carte.

| Service luciepc | Modèle/capacités | Résidence observée | Décision et raison |
|---|---|---:|---|
| 7878 | granite-278m, dense768 | 6,97 Gio | Conservé chaud : code courant déclaré granite278/768, dont les configurations connues de G et code-principal/lourd. |
| 7879 | BGE-M3, dense1024 et creux ; relecteur/OCR annoncés | 7,67 Gio | Conservé chaud : mémoire réelle déclarée BGE1024, code creux et suites récentes. Pas de ventilation de VRAM par sous-modèle. |
| 7880 | granite-107m, dense384 | 4,91 Gio | Arrêté ; instance passée au démarrage manuel. Aucun flux courant/configuration107/384 ni réservation identifié dans le périmètre inspecté. Relançable pour un besoin compatible. |
| 7882 | granite-278m privé, dense768 | 3,74 Gio | Arrêté ; essais A/B et banc terminés, porteurs rendus et absence de client revérifiée. Recette conservée, pas de restauration automatique. |

7882 arrêté à 18:44:52 par TERM sur le seul PID3431955 identifié après contrôle
starttime/SHA/configuration/clients : gain **4 018 868 224 octets, 3,743 Gio**.
Les trois autres services ont gardé leurs PID et leur santé.

107m arrêté à 18:54:16 par `systemctl --user stop` de son instance, pour tenir
compte de `Restart=always`. Son unique dépendance de démarrage était
`default.target` ; l'instance seule est passée de enabled à disabled, sans
masque ni édition du modèle ou de son environnement. PID1409444/port7880
absents, unité inactive/MainPID0 : gain **5 254 791 168 octets, 4,894 Gio**.
7878/PID966256 et 7879/PID3415586 gardent leurs identités et leur santé.

Au total : **9 273 659 392 octets libérés, soit 8,637 Gio**.
Card2 passe de 25 680 760 832 à 16 407 101 440 octets utilisés, sur
34 208 743 424 ; environ **16,58 Gio libres** au relevé final. Poids, caches
de fichiers, configurations et données conservés ; aucun calcul GPU ni
réindexation ajouté pour l'audit.

## Portée des preuves

Le manifeste réel `~/.config/rag3weaver/memoire-longue/backends/memory/backend.json`
déclare BGE1024 ; code/atelier et les configurations connues de code-principal/lourd
déclarent granite278/768. Les passes récentes mémoire/arbre utilisaient BGE7879.
Archi ne conserve aucun consommateur de modèle de son lot terminé.

G du portable est vivant ; ses FD montrent notamment les bases code, atelier
et mémoire ouvertes. G à luciepc est inactif, sans réveil pour l'audit.
L'API d'état refuse l'accès non authentifié ; aucun jeton lu. Les manifestes
connus ont été relus sur les seuls champs non sensibles. Ces déclarations et
chemins ouverts **ne certifient pas le modèle des vecteurs stockés ni l'absence
mondiale d'un index384**. Aucun catalogue persistant ouvert à côté de G.
La décision107 est réversible et fondée sur les flux identifiés ; elle ne
remplace jamais ses vecteurs par ceux d'un autre modèle.

Les cinq connexions7879 appartiennent au processus serveur lui-même ; elles ne
prouvent ni cinq clients externes ni un usage fonctionnel. Les deltas moteur
nuls sur cinq secondes sont des instantanés, pas un historique d'utilité.
Aucune attribution de VRAM par sous-modèle BGE n'est revendiquée.

## Réutiliser un modèle froid

Pour un traitement qui requiert réellement granite107/384, l'opérateur peut
lancer l'instance conservée à luciepc :

```bash
systemctl --user start rag3weaver-embeddings@granite-107m.service
```

Contrôler santé/identité384, périphérique et capacité avant usage. Le démarrage
manuel ne réactive pas l'automatisme de connexion de session. La recette privée7882,
ses quatre paramètres et le SHA du binaire restent chez optimiseur pour un
éventuel besoin dédié. **La réactivation automatique à la demande n'est pas
encore raccordée.**

## Chauffe et conception

Le socle source charge embedder et éventuels relecteur/OCR avant écoute ; il
chauffe des formes de l'embedder puis conserve les objets. ModelSource résout
identités/capacités ; Serveur/Attache savent sonder, lancer et attendre, avec
arrêt limité à leur enfant. Le fournisseur service joint une adresse sans
activation distante. Manquent acquisitions partagées, politique d'inactivité
et reprise à froid commune aux clients.

[Vision de résidence et préchauffage](../../orchestration/visions/2026-11-12-residence-des-modeles-et-prechauffage.md),
[suivi de mise en œuvre](../tickets/2026-11-12-gerer-activation-et-residence-des-modeles.md).
Rendre la chauffe observable par capacité, en distinguant disponibilité
fonctionnelle et formes effectivement chauffées. Aucune nouvelle boucle d'agent
engagée par cette conception.

## Preuves et suites

À luciepc, `~/.cache/optimiseur-notes/audit-residence-12nov/` : inventaires
avant/après, dechargement.json, unites.txt, recette-retour.json,
backends-7970-metadonnees.json ; sous-dossier107m avec avant.json,
recette-retour.json et resultat.json. Le helper107 a eu une erreur de syntaxe
avant toute opération : log conservé, correction et vérification syntaxique
avant l'opération réussie, sans échec runtime masqué.

Copie fidèle et manifeste SHA256 vérifié sur le portable :
`~/.cache/orchestration-notes/audit-residence-modeles-12nov/operateur-luciepc/`
et `preuves-operateur-sha256.json`, sans écrasement d'une preuve antérieure.

Des copies notebook/validated-result déclarent BGE1024 sur7878, actuellement
granite278 ; activation non vérifiée. [Ticket de configuration](../../everything-declarative/tickets/2026-11-12-des-manifestes-bge-visent-le-port-granite.md).
Le suivi historique des redémarrages278 reçoit le relevé de métadonnées du jour,
sans cause de panne inventée ni nouveau redémarrage de ce service.
