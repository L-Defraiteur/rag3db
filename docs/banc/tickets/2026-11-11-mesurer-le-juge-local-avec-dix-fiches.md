# Mesurer le juge local avec dix fiches

- **État** : ouvert — passage arrêté avant inférence : ROCm ouvre aussi un contexte sur card0 malgré le filtre ; restauration 7882 en cours.
- **Famille** : produit
- **Gravité** : coût — latence et qualité du classement à dix fiches non mesurées.
- **Atteignable en service** : non pour cette mesure ; le branchement du juge revient à la session mémoire.
- **Touche rag3weaver** : oui, choix du juge du rappel de mémoire.
- **Pour** : banc
- **Ouvert le** : 11 novembre 2026

## Travail et preuves

Le rapport `docs/banc/11-11-2026/01-rapport-de-session.md`, sections 6 et 7,
demande la mesure k = 10. La page `docs/banc/11-11-2026/03-le-juge-en-gguf.md`
ne donne pour dix fiches qu’une estimation de 1,5 s. À cinq fiches, elle rapporte
0,76 de bonnes premières et 1 076 ms par cas en Q8_0 avec cache ; ces résultats
historiques n’ont pas été rejoués lors de cette reprise.

Lecture locale vérifiée le 11 novembre : `paires2-2000.jsonl` contient 440 paires ;
`paires2.json` décrit 88 cas et 88 fiches distinctes, avec 88 positifs,
176 négatifs proches et 176 négatifs aléatoires. Ces fichiers vivent dans
`~/.cache/rag3db-banc-notes/decision/` du portable.

## Protocole à exécuter

Conserver les cinq candidats de chaque cas et ajouter cinq négatifs distincts
d’autres sessions, avec la graine 20261108. Conserver contextes à 2 000 jetons
et partitions de test ; figer le manifeste et l’ordre avant toute mesure.
Cette extension mesure un groupe construit, pas la distribution d’un rappel réel.

Comparer cinq et dix candidats sous Q8_0 avec `cache_prompt`, groupes contigus :
bonne fiche première, présence dans les deux premières, AUC intra-cas,
latences par cas p50/p95, cache effectivement réutilisé et VRAM avant/après.
Garder les résultats par cas et les commandes dans des fichiers neufs.

Utiliser le llama.cpp b9590 déjà bâti à luciepc, sans le rebâtir, sous `poste mesure` (fenêtre exclusive coordonnée).
Vérifier que le port 8099 est libre. Consigne de Lucie du 12 novembre :
card0/seat1 (PCI 0000:07:00.0, ROCm GPU[1]) est interdite aux sessions.
Utiliser exclusivement card2 (PCI 0000:04:00.0, ROCm GPU[0]) avec
`ROCR_VISIBLE_DEVICES=0` explicite. Lire sa VRAM avant le banc et coordonner
la charge avec les autres sessions. Les quatre services 7878/7879/7880/7882
occupent environ 25,9 Go selon le relevé transmis par Lucie. Un cinquième
processus GPU a provoqué un reset de ring le 9 novembre : ne pas ajouter
le banc aux quatre services. Coordonner avec le porteur du service privé 7882
son arrêt temporaire et sa remise en route, puis relire la VRAM avant chargement.
Ne jamais utiliser card0 comme débordement. Arrêter les seuls processus lancés
par ce banc. Aucun calcul GPU n’a été lancé lors de cette reprise.

## Attente et reste à faire

Lucie autorise le banc sur card2 le 12 novembre sous réserve de coordination
avec les autres sessions et de capacité disponible. Cette consigne remplace
l’ancienne attente d’accord card0. La disponibilité de 7882 et la fenêtre de
charge restent à convenir avec l’orchestration ; aucun tour de push de code accordé.

Construire et contrôler les 880 paires, puis, dans la fenêtre convenue, jouer les mesures,
analyser les résultats et compléter la page GGUF pour la session mémoire.

## Pour fermer

Résultats k = 5 et k = 10 réellement joués, commandes et preuves conservées,
page GGUF actualisée avec les limites du jeu. Déplacer ce ticket vers
`docs/tickets/closed/` et inscrire son état de clôture avec la preuve dans le même commit.

## Consigne directe de Lucie et budget du 12 novembre

Consigne reçue directement dans la session banc, le 12 novembre :

> tu peux utiliser mais donc pas celui de seat1 qui me sers aux jeux videos,
> et en accord avec les autres sessions codex quand elles ont elles aussi besoin
> (selon charge biensur j'ai des gros gpus on peut quand meme y aller)

Le tableau fourni avec cette consigne réserve card0/PCI 0000:07:00.0 à seat1
et désigne card2/PCI 0000:04:00.0 pour les sessions ; il demande de lire la VRAM
et de libérer le privé 7882 si nécessaire plutôt que déborder.

Identité relue par SSH, sans chargement de modèle :
`ROCR_VISIBLE_DEVICES=0 rocminfo` ne montre qu’un agent GPU, gfx1201,
UUID `GPU-e168137a071d0e36`, BDFID 1024 (bus 04, device 00, fonction 0).
Cet UUID correspond à card2 selon le relevé de l’orchestration ; le lien sysfs
card2 mène à PCI 0000:04:00.0. Card0 reste exclue même si elle est libre.
Le service 7882 est PID 912225, démarré le 8 novembre, et n’a pas été arrêté.
`poste --qui` était libre lors du contrôle ; ce constat doit être relu au lancement.

Budget proposé, à confirmer avec le porteur avant lancement :

- un unique llama-server Q8_0, `-ngl 99 -c 16384 -np 1`, port 8099 ;
- poids 4,28 Go ; précédent banc : environ 6,7 Go de VRAM supplémentaire
  serveur et cache compris (page GGUF, 10,8 vers 17,5 Go) ;
- plafond opérationnel proposé : 8 Gio supplémentaires ; exiger au moins
  9 Gio libres sur card2 après libération coordonnée de 7882, soit 1 Gio
  de marge au-delà du plafond, et relire la charge des autres consommateurs ;
- relever la VRAM avant/après chargement et pendant les deux séries ;
  arrêter le serveur du banc si sa hausse dépasse 8 Gio ou si la marge
  restante descend sous 1 Gio. Ce contrôle n’empêche pas un pic instantané :
  la disponibilité initiale et la coordination restent nécessaires ;
- 440 + 880 = 1 320 requêtes séquentielles, cache activé, sans génération
  au-delà d’un jeton, un seul créneau `poste mesure` exclusif ;
- estimation historique : environ 95 s pour k=5 et 132 s pour k=10,
  hors chargement et variation de charge. Limite totale proposée : 15 minutes
  pour les deux séries, plus 2 minutes de démarrage et 3 minutes de remise
  en état, soit une fenêtre demandée de 20 minutes. Le délai commence après acquisition
  de `poste mesure` ; l’attente du verrou ne consomme pas ce budget ;
- pas de reprise partielle pour mesurer les latences : sortie neuve par série,
  cases contiguës et état de cache initial documenté.

Restauration attendue : hotfixs a confirmé à l’orchestration la remise temporaire
de la gestion de 7882 à l’optimiseur pour arrêt et restauration. L’optimiseur
prépare et vérifie la recette exacte de retour avant arrêt. Le banc ne tue ni
ne redémarre ce processus. En fin de banc ou sur échec, arrêter et attendre son
propre llama-server, vérifier port 8099 libéré et VRAM rendue, prévenir le porteur
pour remettre 7882 en route puis faire confirmer santé et disponibilité.
Aucune fermeture du ticket avant mesures et restauration prouvées.

## Ordre et critères d’arrêt

Un unique passage k=5 puis k=10, sans reprise ni retry automatique. Au premier
défaut (santé, requête, résultats incomplets, identité ou budget mémoire),
conserver le diagnostic, arrêter le serveur du banc et suivre la restauration.
Les 20 minutes comprennent 2 minutes maximum de démarrage, 15 minutes communes
aux deux séries et 3 minutes de remise en état ; si la restauration dépasse
cette dernière borne, signaler le blocage et conserver le suivi jusqu’à preuve
de retour en service, sans déclarer la fenêtre rendue.

L’embarquement a annoncé ne pas lancer de nouvelle charge sur ses trois services
pendant la fenêtre proposée. Cela ne prouve pas l’absence d’autres consommateurs :
relire l’activité globale au départ. Les connexions côté serveur ne suffisent
pas à identifier des clients externes. Ne lancer qu’après remise explicite de
7882 par son porteur, contrôle de VRAM et confirmation d’identité card2.

## Opération 7882 préparée par optimiseur, 12 novembre

Porteur du banc inchangé ; optimiseur tient seulement l’arrêt temporaire et la
restauration du service privé. Hotfixs a donné son accord explicite pour SIGTERM
ciblé après GO de l’orchestration, puis retour du même exécutable et de la même
configuration après fin du llama-server. Codeparsers a libéré son usage ; aucun
autre service ne doit être modifié. Aucun arrêt ni chargement effectué au stade
de ce préflight.

Lecture à luciepc à 17:52:36 Paris : PID 912225, starttime `/proc` 49715552,
SHA256 exécutable chargé et installé
`65ec1d1db17d48ee53785cd55e72b9b77ca451d5bc4bb8197a9c1b7d040c595d`.
Aucune connexion TCP établie sur 7882. Santé granite-278m, 768 dimensions,
Flex32, dual/sparse/factice faux, lot conseillé 128/512.

Correspondance GPU prouvée pour ce processus : variable de ciblage `gpu:0`,
journal `Vulkan(DiscreteGpu(0))`, fdinfo amdgpu PCI `0000:04:00.0` avec VRAM
résidente 5 985 464 KiB ; sysfs card2 mène au même PCI. L’autre fd est i915
PCI 00:02.0, sans activité de calcul. Aucune utilisation de card0 autorisée.

Recette de restauration, sans lecture ni copie de fichier env/vault :

```bash
cd ~/.cache/rag3db-mesure-g5/embed-7882
RAG3WEAVER_BURN_DEVICE_EMBEDDER=gpu:0 \
RAG3WEAVER_EMBED_MODEL=granite-278m \
RAG3WEAVER_EMBEDDINGS_ADDR=127.0.0.1:7882 \
RAG3WEAVER_REGIME=plein \
~/.local/bin/rag3weaver-embeddings --adresse 127.0.0.1:7882
```

Script opérateur préparé hors dépôt à luciepc :
`~/.cache/rag3weaver-build/cx-service-7882-banc.py`, copie locale
`~/.cache/optimiseur-notes/service-7882-banc.py`. Le mode `preflight` a été
joué et contrôle identité PID/starttime, argv/cwd, quatre variables utiles,
SHA, santé, GPU et clients. Preuve JSON :
`~/.cache/optimiseur-notes/banc-k10-12nov-7882/preflight.json` ; journal :
`~/.cache/rag3weaver-build/cx-service-7882-preflight.log`.

Modes non encore joués, réservés au GO et à la fin du banc :

```bash
python3 ~/.cache/rag3weaver-build/cx-service-7882-banc.py stop GO-orchestration
# Seulement après arrêt/attente du llama-server, port 8099 libre et VRAM rendue :
python3 ~/.cache/rag3weaver-build/cx-service-7882-banc.py restore
```

L’arrêt revérifie l’identité et les clients juste avant SIGTERM du seul PID
remis, attend au plus 60 s sans force-kill et vérifie le port libéré. Le retour
refuse un port 8099 encore occupé ou un autre occupant de 7882, revérifie le SHA,
relance détaché avec un journal neuf, puis contrôle PID/exécutable/cwd/variables,
santé réelle et fdinfo card2. Les trois services 7878/7879/7880 sont comparés
avant/après, sans arrêt ni changement de configuration. Les JSON d’arrêt/retour
et le journal neuf restent dans le dossier de preuves ; un échec arrête la
chaîne et se signale, sans déclarer la fenêtre rendue.

Reste avant arrêt : GO coordonné, verrou mesure acquis par le banc et activité
relue. Reste après arrêt : capacité card2 suffisante et passage unique du banc ;
après sa fin/son échec, preuve de fin du serveur, VRAM rendue puis restauration
attestée avant restitution du créneau. Le préflight prêt n’est pas un GO.

## Confirmations relayées par l’orchestration le 12 novembre

- Codeparsers : essais A/B terminés, hôtes arrêtés, aucun client, réservation
  ni travail prévu sur 7882. Preuves annoncées dans son ticket fermé require-relatif ;
  confirmation relayée, non revérifiée par le banc.
- Embarquement : aucune charge ni prévision annoncée pendant la fenêtre.
- Hotfixs : remise temporaire de 7882 à l’optimiseur pour arrêt/restauration.
- Orchestration : contrôle indépendant de `/sante` sur 7882 avec granite-278m,
  dimension 768, Flex32 et factice=false ; processus GPU sur PCI 0000:04:00.0,
  consommation annoncée d’environ 5,7 Gio. Ces constats sont relayés et ne
  remplacent pas les contrôles de départ du banc.

L’optimiseur prépare encore recette exacte et retour avant tout arrêt : aucun
GO ni arrêt n’est acquis à cette étape. Il confirme la santé et la disponibilité
de 7882 après restauration. Le seuil minimum de 9 Gio libres sur card2 reste
à lire après libération effective ; la consommation actuelle du service n’est
pas une promesse de capacité. Fenêtre maximale inchangée : 1 200 secondes sous
`poste mesure`, après acquisition. Aucun modèle du banc chargé.

## Passage unique du 12 novembre : arrêt avant inférence

GO coordonné reçu ; `poste mesure` acquis à 17:54:59 Paris, deadline 18:14:59,
lanceur PID 3268337, runner PID 3268353. L’optimiseur a remis 7882 arrêté
à 17:55:25.745 : PID 912225 terminé, port absent, autres trois PID conservés,
11,685 Gio libres sur card2. Sources locales et distantes :
`~/.cache/rag3db-banc-notes/k10-20261112/`.

SHA256 des entrées figées :

- k5 : `eada4788c74f0665208d693b1216d98474d7aed323412dd8d5ca8d5d58013ed0` ;
- k10 : `14e5edb5ae32937a5efcf7cd22bf425796a42cc9cdac095d5ce5194f8cda92eb` ;
- manifeste : `c81463e7fba674878c810d7491813008b17cc26cf30ba3f55fd100bb23570912` ;
- runner : `72b57e0ae242df5ae49d119322315bf2036b6381a92d8a28cb3d0614651ceaf3`.

Identité sysfs et ROCm revérifiée, baseline card2 21 661 941 760 octets.
Serveur PID 3271346 lancé à 17:55:47 avec `ROCR_VISIBLE_DEVICES=0`, puis
arrêté à 17:55:50 par la garde `fdinfo GPU incorrect`, avant lancement du client.
Aucune des 1 320 requêtes n’a été jouée ; aucune mesure de classement ni latence
n’est disponible. Aucun retry n’a été lancé.

`serveur-fdinfo.txt` établit deux ouvertures amdgpu :

- PCI 0000:04:00.0/card2 : 6 783 872 KiB de VRAM résidente (modèle/cache) ;
- PCI 0000:07:00.0/card0 : 40 KiB de VRAM et 2 080 KiB de GTT résidents.

Le filtre ROCm réduit les agents visibles mais n’empêche donc pas cette
ouverture de contexte sur seat1 dans ce bâti. La garde a refusé cette ouverture.
Ce constat ne prouve pas un calcul du modèle sur card0 ; aucune inférence
n’a été demandée. Il interdit néanmoins de déclarer card0 totalement exclue
avec la configuration actuelle. Ne pas relâcher la garde pour faire passer le banc.

`modele-decharge.json` : serveur terminé avec code 0, port 8099 absent,
card2 revenue à 21 665 333 248 octets (environ 3,2 Mio au-dessus de la baseline),
pic 28 731 637 760 octets (hausse environ 6,58 Gio). La restauration de 7882
a été demandée à l’optimiseur ; verrou mesure conservé jusqu’à preuve de retour.
Les preuves fdinfo, journal serveur et déchargement ont aussi été copiées
sur le portable dans le même dossier relatif.

Reste : attester la restauration 7882, puis établir un mécanisme qui interdit
l’accès au périphérique seat1 pour ce processus avant tout éventuel nouveau
passage coordonné. L’autorisation card2 ne vaut pas autorisation card0, même
pour un contexte minuscule. Le ticket reste ouvert ; aucun résultat k10 inventé.
