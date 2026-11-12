# Mesurer le juge local avec dix fiches

- **État** : ouvert — comprendre le contexte ROCm et vérifier l’absence de charge de calcul du banc sur seat1 ; 7882 restauré.
- **Famille** : produit
- **Gravité** : coût — latence et qualité du classement à dix fiches non mesurées.
- **Atteignable en service** : non pour cette mesure ; le branchement du juge revient à la session mémoire.
- **Touche rag3weaver** : oui, choix du juge du rappel de mémoire.
- **Pour** : banc
- **Ouvert le** : 11 novembre 2026

## État actuel au 12 novembre, après passage

Banc arrêté avant inférence ; 7882 restauré et revérifié, verrou mesure rendu.
Zéro paire mesurée et aucune relance. Le travail restant est l’exclusion stricte
de seat1, puis une nouvelle fenêtre coordonnée pour mesurer k10. Les sections
préparatoires ci-dessous sont historiques ; les contrôles de retour sont consignés
à la fin. Le résultat Q8_0 du 11 novembre n’a pas été rejoué.

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

## Préparation historique et attente initiale

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

## Passage du 12 novembre : arrêt et retour 7882 réalisés

Après GO conditionnel de l’orchestration et signal du banc, verrou mesure acquis
17:54:59 Paris, porteur PID 3268337 / runner 3268353, deadline 18:14:59.
Relecture indépendante de `poste --qui` avant l’arrêt.

À 17:55:25, identité PID 912225/starttime/SHA/argv/cwd/variables et absence de
clients revérifiées ; SIGTERM ciblé puis PID terminé et port 7882 absent.
Arrêt attesté à **17:55:25.745**. Autres services 7878/7879/7880 inchangés,
PID respectifs **966256 / 3415586 / 1409444**.
Card2 après libération : total 34 208 743 424 octets, utilisés 21 661 892 608,
libres 12 546 850 816, soit **11,685 Gio libres**. Remise explicite envoyée au banc.

Le banc a abandonné au premier défaut de sa garde d’identité GPU fdinfo,
**avant toute paire**, sans retry. Son serveur PID 3271346 a terminé avec code 0,
port 8099 libre et `modele-decharge.json` à 17:55:50. Aucun résultat k=5/k=10
ne découle de ce passage ; le ticket reste ouvert chez banc.

Avant restauration, optimiseur a vérifié indépendamment l’absence de
`/proc/3271346`, le port 8099 libre et la VRAM card2 revenue à
21 661 892 608 octets, exactement la baseline de sa propre mesure après arrêt.

Restauration lancée **17:56:18.176**, retour VERT **17:56:22.790**, nouveau
PID **3273544**, starttime **84847328**, SHA inchangé
`65ec1d1db17d48ee53785cd55e72b9b77ca451d5bc4bb8197a9c1b7d040c595d`.
Même cwd, argv et quatre variables de la recette. Santé granite-278m, dim 768,
Flex32, dual/sparse/factice faux, lot 128/512. FD amdgpu PCI 0000:04:00.0/card2
avec VRAM résidente 3 924 676 KiB ; aucun fd VRAM card0. Les trois autres services
conservent leurs PID et listeners. Santé relue après fin du lanceur SSH.
Le fichier historique `embed-7882/pid` a été sauvegardé puis actualisé au nouveau
PID ; ancien fichier conservé sous `pid-historique` dans les preuves.

Preuves supplémentaires à luciepc :

- `~/.cache/optimiseur-notes/banc-k10-12nov-7882/juste-avant-arret.json` ;
- même dossier : `arret.json`, `vram-apres-arret.json`, `retour.json`,
  `restauration-service.log`, `pid-restauration`, `pid-historique` ;
- `~/.cache/rag3weaver-build/cx-service-7882-arret.log` et `cx-service-7882-retour.log` ;
- preuve du banc : `~/.cache/rag3db-banc-notes/k10-20261112/modele-decharge.json`.

Retour transmis au banc, à hotfixs et à l’orchestration pour libération du verrou
après leur contrôle. Aucun autre service modifié, aucun fichier env/vault lu
ou copié et aucune limite relâchée. Suite de mesure et diagnostic de la garde
restent à cadrer par banc/orchestration ; aucune relance autorisée par ce bilan.

## Confirmations reçues avant le passage du 12 novembre

- Codeparsers : essais A/B terminés, hôtes arrêtés, aucun client, réservation
  ni travail prévu sur 7882. Preuves annoncées dans son ticket fermé require-relatif ;
  confirmation relayée, non revérifiée par le banc.
- Embarquement : aucune charge ni prévision annoncée pendant la fenêtre.
- Hotfixs : remise temporaire de 7882 à l’optimiseur pour arrêt/restauration.
- Orchestration : contrôle indépendant de `/sante` sur 7882 avec granite-278m,
  dimension 768, Flex32 et factice=false ; processus GPU sur PCI 0000:04:00.0,
  consommation annoncée d’environ 5,7 Gio. Ces constats sont relayés et ne
  remplacent pas les contrôles de départ du banc.

À ce stade de préparation, l’optimiseur préparait encore recette exacte et retour :
aucun GO ni arrêt n’était acquis. Les sections de passage et de retour ci-dessus
attestent les opérations ensuite réalisées. Il confirme la santé et la disponibilité
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

Au moment de cet arrêt, il restait à attester la restauration 7882 (faite et
revérifiée ci-dessous), puis à établir un mécanisme qui interdit
l’accès au périphérique seat1 pour ce processus avant tout éventuel nouveau
passage coordonné. L’autorisation card2 ne vaut pas autorisation card0, même
pour un contexte minuscule. Le ticket reste ouvert ; aucun résultat k10 inventé.

## Restauration attestée et diagnostic statique final

L’optimiseur confirme le retour de 7882 à 17:56:22.790 Paris, PID 3273544,
starttime 84847328, même SHA256/configuration et trois autres PID inchangés.
Preuve opérateur : `~/.cache/optimiseur-notes/banc-k10-12nov-7882/retour.json`
à luciepc. Le banc a revérifié `/sante` (granite-278m, 768, Flex32, factice=false),
PID vivant et fdinfo PCI 04:00.0 ; ancien llama PID absent et 8099 libre.
Preuve : `retour-verifie.json` dans le dossier k10. Marqueur de retour écrit
après ces contrôles ; `fin.json` atteste restauration confirmée, échec du banc
conservé, fin à 17:57:00 Paris, soit 121,13 secondes après acquisition.
`poste --qui` relu libre : aucune mesure, aucun lourd. Créneau rendu.

Condition exacte ayant arrêté le runner :

```python
if '0000:04:00.0' not in gpuinfo or '0000:07:00.0' in gpuinfo:
    raise RuntimeError('fdinfo GPU incorrect')
```

C’est le second terme qui est vrai : PCI 07:00.0 existe dans les fdinfo bruts.
Ce n’est ni une absence de clé ni un format PCI mal interprété. Le serveur
venait de répondre santé 200, mais aucune requête d’inférence n’avait été envoyée.
Les journaux conservent PID, argv et horodatages ; `/proc/PID/exe`, starttime et
numéros exacts des FD n’ont pas été capturés pendant sa vie : ne pas les inventer
après arrêt. Les contenus fdinfo bruts sont conservés. SHA256 relu du binaire
installé après arrêt (ne remplace pas une empreinte du processus vivant) :
`0f4196a018f8aeeb87dc8162f315b8466aefc5815197183429d324fd15ec83c7`.

Aucun nouveau chargement ni nouveau passage n’est autorisé par cette analyse.
La suite reste portée par ce ticket : établir un ciblage qui exclut réellement
l’accès seat1, puis convenir d’une nouvelle fenêtre avant toute mesure k10.

## Proposition statique d’isolation, non exécutée

Lecture seule à luciepc après restauration : les liens `/dev/dri/by-path` et
sysfs associent seat1/PCI 07:00.0 à `card0`, `controlD64`, `renderD130` ;
card2/PCI 04:00.0 à `card2`, `controlD66`, `renderD129`. `bwrap` est installé.
La présence de ce binaire ne prouve ni permission d’espace de noms utilisateur,
ni compatibilité ROCm, ni exclusion par `/dev/kfd`.

Proposition à qualifier : une frontière d’accès aux périphériques, propre au
processus du banc, qui n’expose que les nœuds de card2 nécessaires au backend,
et masque/refuse tous ceux de PCI 07:00.0. Conserver le filtre ROCm explicite
et la garde fdinfo comme contrôles supplémentaires. Un espace de montage privé
avec liste positive de périphériques est un candidat ; une politique de
périphériques du système en est un autre, sous réserve des droits disponibles.
Aucune modification globale de permissions, aucun arrêt de seat1, aucun sudo
ni modification de configuration système n’est proposé sans décision de Lucie.

Avant nouveau passage : qualifier, sur plan et contrôles sans chargement GPU,
les alias, les descripteurs hérités et la portée de `/dev/kfd` ; établir comment
prouver qu’aucun contexte du PCI 07:00.0 n’est ouvert. Une simple liste de nœuds
DRM visibles n’est pas encore une preuve que KFD n’expose pas l’autre carte.
Le mécanisme peut aussi empêcher ROCm de démarrer : refuser proprement vaut mieux
qu’un accès seat1. Ni cette proposition ni l’installation de bwrap n’autorisent
une nouvelle sonde GPU. Prochaine activation seulement dans une nouvelle
fenêtre explicitement coordonnée, avec la garde actuelle conservée.

## Clarification directe de Lucie après publication du 12 novembre

Lucie précise que les 40 Ko ne sont pas en eux-mêmes un problème : elle demande
pourquoi ce contexte existe, et veut surtout que son GPU de jeu ne soit pas
chargé au point de l’empêcher de jouer. Cette précision remplace l’interprétation
précédente qui exigeait zéro ouverture de périphérique seat1. Le travail reste
porté par ce ticket, sans doublon : comprendre l’initialisation puis vérifier
que poids, cache et calcul du banc restent sur card2. Un contexte minuscule
et stable peut être accepté ; il ne suffit pas à prouver l’absence de calcul.

Recherche en cours : sources locales llama.cpp d2462f8 et ROCm 7.2.4,
documentation primaire AMD, puis proposition de contrôle des compteurs par
processus/PCI. Ne pas confondre charge globale card0 (le jeu peut tourner) et
charge imputable au banc. Aucune nouvelle charge GPU à cette étape ; toute
mesure reste coordonnée avec l’orchestration et les porteurs des services.

Correction de vocabulaire : le journal serveur conservé mentionne un warmup
interne avant santé 200. Zéro requête du banc et zéro paire mesurée sont établis ;
« aucune inférence » ne doit pas être interprété comme preuve d’absence de
calcul interne au démarrage. Le PCI de ce warmup n’a pas été mesuré par des
compteurs d’exécution dans le passage arrêté.

## Explication sourcée : initialisation avant sélection des GPU visibles

Paquets lus à luciepc : rocm-core, hsa-rocr et hip-runtime-amd 7.2.4-1.1.
Sources primaires examinées au tag ROCm `rocm-7.2.4`, sans charger de modèle :

- [topology.c, vérification des nœuds](https://github.com/ROCm/ROCR-Runtime/blob/rocm-7.2.4/libhsakmt/src/topology.c#L648) :
  le runtime ouvre le nœud DRM pour déterminer les GPU disponibles ;
- [fmm.c, initialisation des espaces mémoire du processus](https://github.com/ROCm/ROCR-Runtime/blob/rocm-7.2.4/libhsakmt/src/fmm.c#L2790) :
  parcours des nœuds GPU, ouverture de leur render node, puis acquisition
  de leur espace mémoire auprès de KFD (`acquire_vm`, vers la ligne 3058) ;
- [amd_topology.cpp, découverte et filtre](https://github.com/ROCm/ROCR-Runtime/blob/rocm-7.2.4/runtime/hsa-runtime/core/runtime/amd_topology.cpp#L330) :
  récupération des propriétés avant application de `ROCR_VISIBLE_DEVICES`,
  puis distinction des agents exposés et désactivés. Des objets internes
  existent aussi pour les GPU masqués ; ils ne sont pas tous exposés à HIP ;
- [documentation AMD sur la visibilité](https://rocm.docs.amd.com/en/latest/reference/system-optimization/gpu-isolation.html) :
  les variables de visibilité définissent les GPU présentés à l’application,
  distinctement d’une isolation des périphériques par le système.

Le chemin d’initialisation explique donc l’ouverture de PCI 07:00.0 malgré
le filtre, sans nécessiter d’y placer le modèle. Les 40 KiB observés sont
compatibles avec ce petit état de gestion mémoire ; leur allocation précise
n’a pas été attribuée individuellement. Les poids/cache étaient établis sur
PCI 04:00.0 par les fdinfo. Le code local llama.cpp d2462f8 énumère les GPU
visibles et interroge leurs propriétés et mémoire au démarrage. Aucun nouveau
runtime ou modèle n’a été lancé pour cette analyse.

## Critère révisé, à qualifier avant la nouvelle fenêtre

Selon la précision de Lucie, ne plus faire rougir sur la seule existence d’un FD
PCI 07:00.0 ou du contexte stable de 40 KiB. L’objectif est de préserver son jeu :
modèle, cache et exécution du banc sur card2, aucune charge de calcul du banc
sur card0, petite mémoire de gestion tolérable. Ce critère remplace les mentions
précédentes exigeant zéro ouverture de seat1 ; l’historique du passage reste exact.

Conserver une baseline par processus et PCI avant warmup, suivre la mémoire du
banc sur card0 pendant warmup et requêtes, et vérifier les compteurs d’exécution
par GPU du processus si le pilote les expose. Dédupliquer les fdinfo par
`drm-client-id` pour ne pas compter deux fois le même client. Un compteur absent
ne signifie pas zéro calcul. La charge globale card0 ne permet pas seule
l’attribution : le jeu et le bureau de Lucie peuvent légitimement l’utiliser.
Si les compteurs nécessaires ne sont pas disponibles, documenter cette limite
et choisir un moyen d’attribution avant de promettre l’absence d’impact.

Les plafonds du contexte toléré et la manière de détecter une activité imputable
au banc restent à qualifier ; ne pas inventer une garantie de fluidité depuis
un seul instantané de mémoire. Pas de changement global de permission GPU,
pas de sudo ni de sandbox compliqué uniquement pour supprimer 40 KiB.
La mesure k10 attend une nouvelle fenêtre coordonnée, avec arrêt/restauration
7882 préparés comme précédemment. Le ticket reste ouvert jusqu’aux mesures réelles.

## Recette de garde proposée, statique et à relire

Code de capture préparé sur le portable, sans exécution contre un processus GPU :
`~/.cache/rag3db-banc-notes/contexte-rocm/garde_charge.py`.
Seule vérification réalisée : compilation syntaxique Python ; ce n’est pas
une validation runtime ni une preuve d’innocuité du banc.

La capture prend explicitement PID et starttime attendu ; elle revérifie le
champ 22 de `/proc/PID/stat` avant/après, conserve chemin et SHA256 de
`/proc/PID/exe`, argv, les seules variables de visibilité GPU, tous les numéros
et cibles `/proc/PID/fd/N` et contenus `/proc/PID/fdinfo/N`. Un PID réutilisé,
FD disparu ou unité inconnue refuse la capture. Les clients amdgpu sont dédupliqués
par `(drm-pdev, drm-client-id)` ; deux vues divergentes rendent la capture invalide.

Champs et unités : `drm-resident-vram` et `drm-resident-gtt` en KiB, convertis en
octets ; repli sur `drm-memory-vram/gtt` si disponibles ; absence conservée comme
inconnue. Les champs `drm-engine-*` en nanosecondes sont conservés sans inventer
ceux que le pilote ne fournit pas. Les instantanés du passage arrêté ne contiennent
pas ces compteurs d’exécution : on ne peut pas en déduire un zéro de calcul.

Plan d’intégration à la prochaine fenêtre, non activé :

1. Acquérir `poste mesure`, remise et arrêt 7882 par optimiseur, mêmes contrôles
   de capacité card2 et restauration que le passage précédent.
2. Démarrer le serveur avec `ROCR_VISIBLE_DEVICES=0` explicite et `--no-warmup`,
   pour pouvoir capturer après chargement/santé mais avant toute requête calculée.
   Vérifier PCI, GPU visible unique et poids/cache sur card2. Conserver le SHA
   et la commande exacte ; ne pas modifier le clone llama.cpp de Lucie.
3. Capturer une baseline complète puis à chaque seconde pendant la première
   requête (qui provoque alors l’échauffement) et pendant k5 puis k10. Sorties
   neuves ; pas de reprise partielle et aucune relance automatique.
4. Budget proposé pour le seul processus du banc sur seat1 : au plus 16 Mio
   de VRAM et 16 Mio de GTT (les observations étaient 40 KiB et 2 080 KiB).
   C’est une borne opérationnelle conservatrice proposée, pas un seuil de
   fluidité mesuré. Toute croissance au-delà arrête le serveur du banc.
5. Pour chaque client seat1, différence de compteurs `drm-engine-*` entre
   baseline et captures : toute hausse positive indique une activité imputable
   au processus et arrête le banc. Une baisse, un nouveau client, une disparition
   de champ ou une capture incomplète impose un diagnostic ; ne pas soustraire
   des compteurs provenant de clients différents. Vérifier également une activité
   d’exécution sur card2 quand ces compteurs sont disponibles.
6. Compteur absent : état INDETERMINE, jamais « aucun calcul ». Sans nouvelle
   exécution, aucune preuve rétrospective alternative ne permet d’attribuer le
   warmup passé. Les sources du filtre et les allocations établissent le choix
   du modèle et l’initialisation, pas une mesure d’activité de chaque moteur.
   Pour une prochaine qualification, prévoir une trace HIP/HSA ou un profilage
   AMD reliant les dispatchs à l’agent PCI/UUID ; présence, format et coût de
   l’outil restent à vérifier avant de proposer sa commande. Ne pas promettre
   l’absence de charge à partir du FD seul ou du pourcentage global card0.
7. Limites totales à reconfirmer dans le GO : 1 200 s après acquisition,
   démarrage 120 s, deux séries 900 s communes, arrêt/restauration 180 s.
   Contrôleur arrête uniquement le groupe du serveur et des clients qu’il a
   lancés, attend leur fin, prouve 8099 libre et mémoire rendue, puis demande
   le retour 7882 à l’optimiseur. Retour santé/PID/PCI avant rendu du verrou.

Le petit script ne lance ni ne tue de processus et ne met pas en œuvre tout
ce contrôleur : il rend la capture et ses inconnues reviewables. Le rapprochement
des séries, l’arrêt du groupe et l’intégration au lanceur restent à qualifier.
Pas de nouveau GPU ni de sonde de calcul exécutés pour cette préparation.

## Placement explicite : chemin allocation et calcul relu, sans runtime

Révision locale relue à luciepc : llama.cpp
`d2462f8f7ac6d80070a587ffebf6cd73730f4280` (le bâti existant).
La sélection explicite permet une preuve positive du placement plutôt que
l’exigence injustifiée d’aucun FD sur le GPU masqué :

- [common/arg.cpp, parse_device_list](https://github.com/ggml-org/llama.cpp/blob/d2462f8f7ac6d80070a587ffebf6cd73730f4280/common/arg.cpp#L833) :
  `--device` sélectionne par nom de backend et renseigne `params.devices` ;
- [src/llama.cpp, préparation des périphériques](https://github.com/ggml-org/llama.cpp/blob/d2462f8f7ac6d80070a587ffebf6cd73730f4280/src/llama.cpp#L125) :
  la liste explicite alimente `model.devices` ; `split_mode=none` ne garde
  que l’entrée `main_gpu`. Le journal imprime nom et `props.device_id` PCI ;
- [src/llama-model.cpp, chargement](https://github.com/ggml-org/llama.cpp/blob/d2462f8f7ac6d80070a587ffebf6cd73730f4280/src/llama-model.cpp#L1205) :
  buffers et couches GPU pris dans cette liste ; des couches peuvent rester
  sur CPU, ce qui n’ouvre pas une autre carte graphique ;
- [src/llama-kv-cache.cpp](https://github.com/ggml-org/llama.cpp/blob/d2462f8f7ac6d80070a587ffebf6cd73730f4280/src/llama-kv-cache.cpp#L226) :
  le cache déporté suit le périphérique de la couche ;
- [src/llama-context.cpp](https://github.com/ggml-org/llama.cpp/blob/d2462f8f7ac6d80070a587ffebf6cd73730f4280/src/llama-context.cpp#L256) :
  initialisation des backends GPU de `model.devices`, puis backends ACCEL/CPU ;
- [ggml-cuda.cu, calcul du graphe](https://github.com/ggml-org/llama.cpp/blob/d2462f8f7ac6d80070a587ffebf6cd73730f4280/ggml/src/ggml-cuda/ggml-cuda.cu#L4509) :
  sélection du périphérique du contexte avant exécution. Le backend HIP
  utilise les alias HIP correspondants ; son registre expose le PCI lu par
  `cudaDeviceGetPCIBusId` (vers 5698) via `props.device_id` (vers 5064).

Commande candidate pour la future fenêtre, NON exécutée :

```bash
ROCR_VISIBLE_DEVICES=0 ~/git_workspaces/llama.cpp/build-hip/bin/llama-server   -m ~/.cache/rag3db-banc-notes/hf/qwen3-4b-Q8_0.gguf   --device ROCm0 --split-mode none --main-gpu 0   -ngl 99 -c 16384 -np 1 --fit off --no-warmup -lv 4   --host 127.0.0.1 --port 8099
```

Avant toute paire : exiger que le journal associe le périphérique effectivement
retenu à PCI 0000:04:00.0, backend GPU unique, et que les fdinfo attribuent
modèle/cache à cette carte. Refuser PCI inconnu, liste supplémentaire ou
contradiction source/journal/FD. Les traces de placement du graphe, si disponibles,
complètent cette chaîne ; ce plan n’affirme pas un compteur d’exécution nul.
Le nom logique ROCm0 seul ne prouve jamais la carte physique.

Une absence de compteurs moteur ne déclenche pas un chargement supplémentaire
pour redécouvrir l’inconnu : elle est déjà connue. La chaîne source figée +
sélection explicite + PCI effectivement retenu + allocations sous surveillance
est une alternative de preuve du placement de la charge du modèle, compatible
avec le petit contexte d’initialisation toléré par Lucie. Elle ne mesure pas
chaque éventuelle microactivité du pilote et ne garantit pas des FPS déterminés.
Les contre-mesures restent la borne de mémoire imputable au banc sur seat1
et l’arrêt de son seul groupe au premier écart, sans se fonder sur la charge
globale du jeu. Nouvelle fenêtre et contrôle opérateur encore nécessaires.

## Corrections statiques de la capture

Nouvelle empreinte du script externe `contexte-rocm/garde_charge.py` :
`0da91960271591c35930898844e05af7defd4d602b63bd6d627b7caf124319a9`.

- `drm-engine-capacity-*` est ignoré pour les durées : ce n’est pas une unité ns.
- L’ensemble des FD GPU (`/dev/dri/*`, `/dev/kfd`) est relevé avant/après ;
  sa modification invalide la capture. La disparition d’un FD non GPU,
  notamment HTTP, est consignée mais ne fait plus échouer un ensemble GPU stable.
- Les compteurs absents restent inconnus ; le petit outil de capture ne décide
  toujours pas seul de la validité du placement ni du lancement des requêtes.

Vérifications sur quatre cas synthétiques, sans processus GPU ni lecture GPU :
capacité sans unité ns ignorée, durée compute 42 ns conservée, disparition HTTP
avec ensemble GPU stable tolérée, starttime incorrect refusé. Quatre verts lus.
Harnais externe : `contexte-rocm/verifier_capture.py`. Aucun nouveau chargement,
sonde GPU ou opération sur 7882 ; le contrôleur final reste à relire/intégrer.
