# Relecture indépendante des preuves Vertex CPU

Avis favorable **borné aux sept juges CPU avec serveur factice**, sans nouvelle exécution. Relecture du commit documentaire c599a189b59a56052df4713c135dca1cff7685b3, application c38fc3e4ddd8c806de904c7d08eb43b4988fec99, pilote 441002d11bd07b7e8b77b0351d6e522ac5a9c6db. Le [ticket Vertex](../tickets/2026-11-16-un-modele-vertex-declare-garde-son-jeton-d-une-heure.md) reste ouvert. La qualification d’union et l’intégration relèvent d’une décision distincte de C7.

## Contrôles effectués

La copie locale privée `~/.cache/rag3weaver-build/cx-preuves-ed-vertex-token-green-v1-local` contient les 59 fichiers du manifeste : toutes les tailles et tous les SHA256 ont été recalculés, sans différence. Le manifeste publié a bien le SHA256 `7a73c248158fe1587cb0045db06aca68f63589528b716416e37fbe2b38cba0a6`.

L’ELF copié, lu comme données seulement, fait 195368680 octets, SHA256 `8cbcefea72d6f584cec7e2c87f42eaeda32c9026ba6c737767a200ae65395406`. Le préfixe APP `c38fc3e4d` se trouve à l’offset 8049153. Le ledger recalculé vaut `80d72ff5b456155fa03ee226c218c362a8e2223a6f02db531c5910ba50185e33`. Sa publication observée à 1190179.820097180 précède les sept résultats, de 1190179.932874971 à 1190179.979972287. La vérification sur deux hôtes est attestée par le reçu publié ; cette relecture recalcule la copie locale, sans prétendre avoir refait une observation sur le PC.

Les sept commandes utilisent les noms distincts attendus avec `--exact --nocapture`. Chacun des sept logs contient **1 passed, 0 failed, 0 ignored**, 1954 filtrés ; chacun des résultats associés est rc0. Cela ne qualifie pas la bibliothèque entière. La compilation enregistrée est offline/locked, sans features par défaut, avec `daemon,openai-llm`, `--lib --no-run`, rc0.

Les fichiers archivés `qualify.py` et `vertex_cache.py` sont identiques octet pour octet à ceux du pilote 441002d11b, SHA256 respectifs `e95d506185bd289de68005aa0e1f341a18dfa2d47f4a0f197547b19fbe7ae161` et `bd395bf4832841fb8132366923ad6569fa90a85a0f23dc76e70ac9bae44c4d4d`. Lecture statique seulement. Le pilote contrôle HEAD et propreté de l’application, du pilote et du sous-module, empreintes des outils et bibliothèques avant/après. Les 1936 entrées de fingerprints et le reste du contrat de reprise sont strictement identiques entre configuration préparée et capture après poste. Les deux commandes réelles de garde, ancien propriétaire puis reprise, ont rc0 ; aucune nouvelle capture physique du target n’a été faite ici.

## Sens des juges et bornes

Le code APP montre que `connect_llm` délègue à `connect_llm_with_vertex`. Le juge déclaratif emprunte ce helper avec constructeur du vrai client injecté vers HTTP factice : paramètres projet/région/modèle, origine et contexte sont assertés, ainsi que zéro acquisition à la connexion, une invalidation et la séquence de trois Bearer attendue après deux générations. Il vérifie donc la conservation de la capacité injectée. Il ne passe pas par `TokenSource::from_env`. Les autres juges couvrent le cache factice, le refus persistant après un seul renouvellement, l’absence de renouvellement d’un Bearer fixe, l’absence de rejeu après fragment et le debug expurgé.

L’environnement archivé contient les chemins XDG et état séparés, TMP court et appareils Burn CPU. Le pilote construit une liste d’environnement autorisée et omet le jeton d’autorité du profil publié. La pause de publication est plafonnée à `min(END-30, maintenant+120)` ; chaque commande est plafonnée par `min(END-15, maintenant+cap)`. Le contrôle d’échéance après ouverture et avant publication est présent. Budget commun 1130 s, début 1190067.356433272, END 1191197.356433272 ; composé enregistré 113.024376022 s. Limites enregistrées : 32 Gio, memory.high=max, swap=0, quota CPU cinq cœurs ; jobs effectifs 5.

Le verdict archivé est VERT/code0, erreur nulle, restes vides, nettoyé. Le composé atteste autorité terminée et bus absent. Le contrôle indépendant publié après retour recense 104 identités possédées, aucune vivante, aucun found, aucune exemption, cgroup et bus absents. Les douze UNKNOWN non attribués restent UNKNOWN ; cela ne prouve aucune absence globale d’activité GPU. L’exemption FD du poste ancêtre figure dans les scans internes, puis disparaît dans l’observation après retour.

Les anciens rouges restent distincts. Pas de qualification OAuth réel, credentials, cloud, F300, SDK natif, durabilité des passes, union ni master. Aucune base ouverte, aucun processus inspecté en direct, aucun exécutable lancé, aucun service modifié dans cette relecture.
