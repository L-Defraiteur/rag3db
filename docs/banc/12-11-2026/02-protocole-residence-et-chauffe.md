# Résidence et chauffe : protocole du volet banc, nuit du 12 au 13 novembre

Suivi commun : [activation et résidence](../../embarquement/tickets/2026-11-12-gerer-activation-et-residence-des-modeles.md),
porteur runtime Embarquement, porteur recette/harnais/preuves Banc.
[Plan autorisé](../../orchestration/12-11-2026/02-lots-independants-pour-la-nuit.md).
Ce document prépare la qualification ; aucune tranche de runtime ni mesure
réelle n’est déclarée livrée par sa seule écriture. Le banc Qwen clos ne se rejoue pas.

## Point de départ lu

Base a783013f1. `Serveur::assurer` sonde puis lance avec un délai borné ;
`Attache` distingue processus possédé et service tiers. `DaemonEmbedder::assurer`
le compose, et `ModelSource::service` joint un fournisseur existant.
Le serveur chauffe actuellement l’embedder avant écoute ; son compteur de formes
réussies ne prouve pas l’état de chauffe du relecteur ou de l’OCR.
Les témoins existants d’e2e_demon_embeddings ne prouvent pas l’acquisition
commune sous concurrence de la nouvelle tranche.

Embarquement doit confirmer l’interface réellement appelable et sa révision :
identité de ressource, profil/dimension/capacité, acquisition, événements d’état,
annulation d’un demandeur, libération et mécanisme de reprise. Le harnais utilise
ces primitives ; il ne réimplémente ni un gestionnaire de résidence ni le runtime.
Aucun fichier model_source/serveur/fournisseur n’est édité par Banc.

## Mesures et témoins attendus

Toutes les durées viennent d’une horloge monotone du contrôleur, avec bornes
de processus et identité de génération. Les timestamps du journal fournisseur
restent une autre source ; ne pas soustraire des horloges de postes différents.

| scénario | action sur une fixture possédée | preuve recherchée |
|---|---|---|
| activation partagée | A et B demandent la même identité, barrière commune, B rejoint avant fin de l’activation | un constructeur/lancement, une instance, les deux obtiennent la capacité attendue |
| chauffe observable | le fournisseur publie chargement/chauffe/prêt, avec les formes effectivement traitées | états réels accessibles au demandeur, durée de chaque phase, erreur conservée par capacité |
| annulation isolée | A abandonne pendant l’activation, B reste demandeur | résultat d’A annulé, B reçoit puis utilise la même instance ; aucune destruction de la ressource de B |
| attache chaude | C acquiert pendant qu’un usage garde la ressource prête | même instance, pas de nouveau chargement ; temps d’acquisition rapporté, sans seuil arbitraire de vitesse |
| reprise | tous les usages sont rendus, désactivation explicitement prévue par la politique du scénario, nouvelle demande | nouvelle instance/génération, même identité de modèle/dimension/capacités et réponse valide |
| tiers et identité étrangère | un fournisseur non possédé ou un modèle/dimension incompatibles occupe l’adresse | refus ou attache conforme au contrat ; aucune fermeture d’un processus tiers ni substitution de vecteurs |

Le constructeur factice et les barrières CPU doivent permettre de placer
l’annulation précisément pendant l’activation. Une simple pause temporelle
n’est pas la preuve que B attendait déjà la même activation. Les scénarios
factices qualifient le harnais et le contrat ; ils ne sont pas des mesures de
chargement GPU et ne remplacent pas l’appelant réel du framework.

## Données brutes à conserver

Révision du runtime et du harnais, empreintes exécutables/poids/tokeniseur,
profil et politique déclarée, modèle/dimension/capacité, adresse privée réellement
liée, propriétaire, PID/starttime et génération d’activation. Par demandeur :
instant de demande, début/fin d’attente, annulation, acquisition obtenue et
libération ; par instance : construction, chauffe observable, prêt et arrêt.
Un événement absent reste inconnu, jamais une durée nulle ou un succès vide.

Avant et pendant le réel : environnement GPU ciblé, PCI effectif des poids et
buffers, mémoire du processus et des cartes, consommateurs concernés, journaux
et erreurs exactes. Captures en lecture, sans arrêt de service partagé.
Le petit contexte ROCm sur seat1 n’est pas confondu avec une charge du modèle.

## Conditions avant un essai réel

Un modèle privé déjà disponible et utile est choisi avec Embarquement ;
modèle exact, dimension et formes de chauffe figés avant la passe. Aucun nouveau
téléchargement ni ouverture de base de Lucie. Les services 7878/7879 restent
intacts ; ni 7880 ni 7882 ne sont relancés par défaut. Une instance privée
sur port nul appartient au lot ; sa fin et ses descendants sont vérifiés.

Card2 est le choix normal, selon capacité et consommateurs réels. Une comparaison
cold/hot/reprise prend `poste mesure` et publie son budget après acquisition.
Le budget exact dépend du constructeur et de son contrat ; il doit être figé
avant lancement, avec réserve d’arrêt, premier rouge arrêté, sans répétition
inchangée ni reset silencieux. La reprise prévue par le scénario n’est pas
une relance après un échec : elle n’a lieu que si les étapes précédentes passent.

Card0 n’est pas nécessaire à la préparation. Son éventuel usage réel est limité
au créneau 13 novembre 00:00–08:00 Europe/Paris, sous la garde privée/cutoff
préparée par Optimiseur et restitution avant 07:55. Un filtre seul n’est pas
la garde du créneau. Sans dépendance prête, pas de modèle lancé pour occuper la carte.

## Dépendances encore attendues

Interface et révision du runtime Embarquement, identité du modèle privé et
politique du scénario, événements de chauffe disponibles, mode d’appel public
et propriété de l’activation/arrêt. La préparation du harnais peut avancer
sur fixtures CPU après accord de ce contrat ; la mesure réelle attend ces faits.
Gemini Decision1 reste hors de ce protocole : attente Vertex, aucune autre voie.

## Contrat confirmé par Embarquement, préparation du banc

Branche propriétaire annoncée : `cx-activation-modeles`, base a783013f1,
révision runtime pas encore reçue. `ModelSource.activation` déclare programme,
args/env, profil, périphérique, coordination/journal, budgets activation/attente
et résidence `maintenir`. Le programme reçoit `--adresse H:P` avant ses args.
Premier appelant : `connect_embedder` et sa variante annulable ; capacité dense
seule, autres capacités refusées dans cette tranche. `activation::demander`
rend une demande dont l’état passe par Coordination/Activation/Disponible/Echec ;
l’attente prend le signal d’annulation et rend le bail. Ces formes sont le
contrat annoncé, pas encore une compilation ni un accès runtime qualifiés ici.

La santé exposera un rapport de chauffe optionnel par capacité/forme/erreurs ;
absence de rapport chez l’ancien serveur = inconnue. Le fournisseur factice
ci-dessous garde volontairement la santé historique pour qualifier ce cas,
sans inventer des champs du nouveau contrat avant le code.

Fichiers préparés dans la branche Banc :

- `extension/rag3weaver/tests/fixtures/residence/fournisseur.py` : exécutable
  Python standard, aucun import ML/GPU ; adresse locale privée, génération et
  journal obligatoires, barrières fichiers de chargement et de chauffe ;
- `extension/rag3weaver/tests/fixtures/residence/test_fournisseur.py` : cinq filets
  CPU joués, gates avant écoute, identité explicitement factice, déterminisme,
  erreur de chauffe avant prêt, timeout et refus des entrées invalides.

Commit 3963f6829, contrôle Python avec ResourceWarning en erreur : cinq verts,
aucun processus fixture laissé en vie. Le premier essai avait relevé des réponses
HTTPError non fermées dans le test ; elles sont maintenant explicitement fermées.
Aucun faux résultat runtime n’est déduit de ces tests de fixture. Une paire de
requêtes HTTP ne prouve pas à elle seule une activation partagée.

Modèle réel proposé avec Embarquement : privé granite-107m/384 déjà disponible,
card2, sans arrêter les services courants. Aucun besoin card0 identifié ;
Optimiseur informé. La mesure de reprise devra utiliser une désactivation
explicite du seul privé possédé, si le contrat le permet, car `maintenir` ne
promet ni libération au dernier bail ni éviction. Elle ne dira pas qu’un TTL
ou une activation distante sont livrés. Attente de la révision propriétaire
avant branchement des vrais appels et de leurs témoins.

## Deux vrais processus sur la primitive commune : CPU-01

Sources : branche `cx-banc-residence-appels`, base propriétaire ef11f0e16,
harnais 714fb8a65. Fichiers Banc réservés :
`tests/activation/banc_demandeur.rs` et
`tests/fixtures/residence/banc_interprocessus.py` sous `extension/rag3weaver`.
Le demandeur compile directement le vrai `Serveur` et son module d’activation
par rustc ; il ne remplace pas le registre ou le verrou interprocessus.
Les tests du connecteur `ModelSource` restent chez Embarquement.

Recette exécutée sur le portable, sous `poste lourd`, plafond 32G, timeout 60 s :

```bash
rustc --edition=2021 extension/rag3weaver/tests/activation/banc_demandeur.rs -o <binaire privé>
python3 extension/rag3weaver/tests/fixtures/residence/banc_interprocessus.py   --demandeur <binaire privé> --preuves <dossier neuf>
```

Deux scénarios verts, sans GPU :

- Partage : A en Activation, B en Coordination avant libération des gates ;
  un seul constructeur effectif, A et B obtiennent puis utilisent le fournisseur.
- Annulation : les mêmes états sont observés avant l’annulation d’A ; A reçoit
  une erreur nommée annulée, reste vivant, B obtient puis utilise le fournisseur.
  Les traces d’A montrent ensuite Disponible, sans second constructeur.

La barrière est attestée avant libération, pas déduite d’une pause arbitraire.
Les groupes privés des demandeurs et leur fixture sont arrêtés en fin de cas.
Les journaux du fournisseur attestent l’arrêt. Preuves conservées hors worktree :
`~/.cache/rag3db-banc-notes/residence-nuit-12nov/cpu-01/{partage,annulation}/`,
avec `preuve.json`, journaux fournisseur, traces A/B TSV et stdout/stderr.
Durées du contrôleur environ 0,149 s et 0,132 s ; ce ne sont pas des temps
réels de chargement/chauffe de poids. Aucun benchmark matériel n’en est déduit.

Limites : primitive d’activation, pas encore parcours du connecteur `ModelSource` ;
annulation du demandeur, pas crash du processus qui tient l’activation ; maintien
explicite, pas TTL ni comptage de baux distants. La primitive exige une adresse
locale fixe : le harnais libère un port attribué localement juste avant départ,
et refuse un conflit sans tuer l’occupant ni choisir silencieusement un autre port.

## Identité déclarée de fixture demandée par Embarquement

Commit dédié 319da5275 : seules les variables non secrètes
`RAG3WEAVER_ACTIVATION_PROFIL`, `RAG3WEAVER_ACTIVATION_PERIPHERIQUE` et
`RAG3WEAVER_ACTIVATION_IDENTITE` sont reflétées comme profil/périphérique/recette
quand les trois sont présentes. Le rapport partiel reste absent, la chauffe
reste inconnue et `factice=true`. Sept filets CPU de fixture verts. Ce reflet
ne prouve aucun placement physique et ne charge aucun poids.

Ces sources sont transmises à Embarquement pour ses cas managed d’identité et
ses appelants. Son durcissement du contrat d’identité est en préparation ;
CPU-01 ne le qualifie pas et la mesure GPU attend son parcours connecteur vert.

## Connecteur réellement qualifié par Embarquement après le rouge de fixture

Le premier passage des vrais tests de connecteur a donné un vert et quatre
échéances de 5 s : la fixture rendait du JSON avec espaces, alors que la sonde
du démon exige le marqueur compact `"service":"rag3weaver-embeddings"`.
Le fournisseur réel tiny_http rend déjà ce format. Correction de la fixture
uniquement, sans allonger le budget ni assouplir le runtime.

Embarquement atteste d5133197e (runtime 8a0 et correctif compact) : cinq tests
CPU de connecteur verts, zéro ignoré ; vrai /embed, dimension 384, timeout
préservé et profil étranger d’un autre processus refusé sans second constructeur.
Une vérification CPU de chauffe partielle est verte : 12/16 formes réussies,
quatre erreurs conservées. Preuve propriétaire :
`luciepc:~/.cache/embarquement-notes/activation/cpu-e.log`, unité terminée success.
Ces résultats n’attestent pas encore un chargement du nouveau fournisseur GPU.

Banc a ajouté un filet des octets bruts dans 86c5fc249 : la santé doit porter
exactement le marqueur compact, rester JSON parsable et factice. Huit tests
CPU de fixture verts, ResourceWarning traité en erreur. Les verts de primitive
CPU-01 ne sont toujours pas présentés comme la preuve de ce connecteur.

## Recette prévue pour le privé réel 107m/384, pas encore exécutée

Consommateur proposé : `examples/banc_residence.rs`, commit Banc 678c5e678,
compilable avec daemon, sans modification de Cargo.toml/lib/runtime. Il prend
une déclaration `ModelSource`, observe une demande publique d’activation puis
raccorde le vrai `connect_embedder_annulable` et valide les vecteurs de `embed`.
L’acquisition initiale et le raccordement du connecteur déjà prêt sont mesurés
séparément. On ne les présentera pas comme une seule connexion froide directe.

Poids déjà présents à luciepc, lus et hachés sans GPU :

| artefact | octets | SHA256 |
|---|---:|---|
| ~/.cache/rag3weaver/granite-107m/model.bpk | 428007424 | 7a7c9c559236bec6f5b903f0f2573221d1a9252f5d37f50d63dbd2b347038ca1 |
| ~/.cache/rag3weaver/granite-107m/tokenizer.json | 9081382 | 7fc9e475f2ac473f070052d92b0c46ec2b91d9cd672935feb6fd837a65d33f78 |

Lecture card2 : PCI 0000:04:00.0, 17 212 403 712 octets utilisés sur
34 208 743 424, environ 15,83 Gio libres. Ce relevé n’est pas une réservation ;
il sera refait au départ avec les consommateurs. Aucun service courant arrêté.

Profil prévu : privé granite-107m/384, maintien explicite, card2.
`RAG3WEAVER_BURN_DEVICE_EMBEDDER=gpu:0` correspond au choix Vulkan précédemment
qualifié pour card2, mais le prochain processus doit à nouveau le prouver par
son log et ses fdinfo/allocations. Les variables de sélection sont locales à
sa déclaration ; aucun choix GPU global ni usage card0 prévu.
Les octets de santé activation attestent la déclaration, pas le PCI physique.

Séquence à figer avec le contrôleur avant départ :

1. Fenêtre exclusive MESURE, empreintes du fournisseur nouveau et du consommateur,
   sources/profil/poids/tokeniseur publiés, nouveau dossier et deadline absolue.
2. Deux demandeurs A/B de la même déclaration. Une entrée privée de fixture
   bloque l’exec du fournisseur avant chargement pour observer A en Activation,
   B en Coordination et annuler A précisément ; elle ne remplace pas le runtime.
3. Libérer le lancement réel pour B, garder A vivant jusqu’à fin de B ; prouver
   une seule instance réelle, identité/384/non-factice et rapport de chauffe,
   placement card2, puis vrai embed par le connecteur public.
4. Un client C acquiert à chaud la même déclaration et utilise la même instance,
   sans chargement supplémentaire. Capturer la durée et les PID, pas inventer
   un seuil de vitesse depuis le fait qu’il est à chaud.
5. Après usages, arrêter uniquement le privé possédé, attendre PID/groupe/port
   et mémoire rendus. Une demande D de la même déclaration qualifie la reprise
   en nouvelle instance, sans changer de modèle/dimension ni d’espace utilisateur.
6. Arrêt du second privé, preuves de fin et état inchangé des services courants,
   puis restitution MESURE. Aucun service partagé à restaurer : aucun n’a été arrêté.

La reprise est la seconde activation explicitement prévue, pas une relance
après erreur. Premier rouge inattendu : arrêt du scénario et diagnostic, pas
rejeu inchangé. Proposition de borne globale 1 200 s après acquisition,
activation/attente 300 s par épisode dans la borne commune, réserve de fin 180 s ;
la recette finale doit donner les deadlines de phases sans les additionner
au-delà de cette borne. Budget mémoire proposé : hausse card2 au plus 8 Gio,
marge libre au moins 2 Gio, mémoire imputable au privé sur seat1 sous 16 Mio
VRAM et 16 Mio GTT ; chiffres à relire avec le constructeur. Ce sont des filets
opérationnels, pas une promesse de FPS ni un quota de processus.

Dépendances encore réelles : nouveau fournisseur compilé portant l’attestation
activation (ancien installé inadapté), consommateur compilé sur la même source,
contrôleur et entrée privée figés, revue du plan et fenêtre comparative.
Embarquement prépare le binaire et sa passe ; aucune mesure ni charge GPU
n’a été effectuée par cette préparation.

## Entrée privée et dépendance de bâti, après qualification CPU

Entrée de banc f5ef92d46 : `tests/fixtures/residence/activation_privee.py`
et ses quatre filets CPU, aucun runtime remplacé. Elle reçoit `--adresse` du
vrai gestionnaire, vérifie modèle/périphérique et empreintes des artefacts,
attend un signal JSON délibéré du scénario puis `exec` le nouveau fournisseur.
Le PID, le groupe et l’attestation d’activation sont conservés ; elle ne crée
ni verrou de résidence ni politique d’éviction parallèles.

Douze filets Python CPU passent désormais : huit de fournisseur et quatre
d’entrée privée (barrière avant exec, même PID/groupe après exec, artefact
modifié refusé, signal invalide et deadline avant exec). Les processus de
ces tests sont des fixtures explicitement factices, pas des charges de poids.

Source propriétaire annoncée 356d7d927 inclut le consommateur 678c5e678 et le
filet compact. Embarquement atteste une requalification CPU conforme
rw5c889b : 16 +5 +1 verts, zéro ignoré. Sa passe commune est bornée à 7 200 s,
32G/j8/target32Go : cinq jeux de features verts à l’état reçu, lib large en cours.
Pas de nouveau service ou GPU lancé par Banc dans cette attente.

Embarquement fournira copies immuables et SHA/source/ldd hors target dans
`~/.cache/embarquement-notes/activation/bin-356d7d927/`, fournisseur et
consommateur, après bâti et fin de portée. Le chemin debug du target seul
n’est pas une provenance suffisante pour une mesure qui survivrait à un rebase
ou rebâti. L’ancienne installation reste exclue de cette qualification.
Le contrôleur final, les empreintes de ces exécutables et la fenêtre MESURE
seront figés avant le réel ; le ticket de résidence reste ouvert.

## Contrôleur préparé avant la mesure réelle

Sources de harnais 51a782793 puis 52202603d :
`tests/fixtures/residence/mesurer_prive.py`, `observation_gpu.py`,
`activation_privee.py`. Vingt-deux filets Python CPU verts ; les dix concernant
identité/chauffe/mémoire/nettoyage ont été rejoués après durcissement des groupes.
Aucun acteur GPU n’a été lancé par ces vérifications.

Le contrôle d’appartenance garde PID/starttime et groupes privés. Une fois
le client terminé, l’arrêt de la ressource maintenue doit encore être rattaché
à un PID/starttime de fournisseur observé ; sans identité prouvée, refus de tuer
le groupe et blocage nommé. Les clients rendent leur usage avant l’arrêt forcé
du privé, santé de maintien conservée ; groupe, port et mémoire sont ensuite
attendus dans la même borne. Un port encore visible après exit n’est pas
considéré immédiatement comme libéré.

Deadline globale 1 200 s après acquisition de MESURE. Délais cumulés absolus
pour les barrières initiales : 60/90/100 s pour A présent, B coordonné, A annulé ;
B disponible avant t0+360 ; usages B et C au plus 60 s chacun, également bornés
par t0+420/t0+480 ; arrêt du premier privé dans 60 s. D disponible dans 360 s
et avant fin globale moins 180 s, son usage avant fin moins 120 s ; arrêt final
dans la borne restante. Les budgets source d’activation restent 300 s et
attente 360 s. Aucun de ces délais ne remet le budget global à zéro.

Les captures /proc gardent executable SHA, argv, PID/start, FD et contenus,
PCI/allocation et valeurs manquantes explicites. La première identité est
prouvée avant usage ; un nouveau gate pour D impose de requalifier la nouvelle
instance. Les compteurs absents restent inconnus, pas zéro ; dépassement de
mémoire seat1 ou modification observée de ses compteurs arrête la chaîne.
Modèle/dimension/factice, attestation profil/périphérique/recette et rapport
chauffe embed sont contrôlés. Une chauffe partielle conserve ses erreurs ;
une chauffe annoncée complète avec erreurs est refusée.

Le consommateur 26522eb8c ajoute `gate_usage` optionnel après le bail :
B/D attendent la preuve physique/santé avant leur raccordement et embed mesurés.
Ce delta de harnais ne change pas le runtime. Une nouvelle copie immuable du
consommateur est demandée à Embarquement ; l’ancienne copie reste préservée.
Les binaires fournisseur/consommateur initiaux sont disponibles hors target,
mais le plan final attend le hash de cette nouvelle copie et la revue des bornes.

Le hachage préalable des poids réchauffe potentiellement le cache de fichiers :
« froid » signifie ici nouveau processus et modèle non résident, pas cache disque
ou pilote vierge. Les deux générations de chauffe ne doivent pas être présentées
comme un essai matériel à caches identiques. Aucune sonde de charge sur card0.

## Paquet final et entrée commune, avant premier GPU

Nouveaux artefacts immuables livrés hors target à luciepc :
`~/.cache/embarquement-notes/activation/bin-345a402d0/`.
Source 345a402d099ae2c345e4b37a96159df970a7ee83, SHA fournisseur
`10f44d35479f2a6462a6db5f2dfbad2714f524eff66a27589940e4e5ad44345a`,
consommateur avec gate d’usage
`c171231c0d158fef5bd06755ca7ee22c1cb00a386d1266461ce7883ef2a6fb73`.
SOURCE/ldd/SHA256SUMS conservés ; aucune dépendance librag3db.so observée dans
ces deux ELF, pas d’attestation native inventée.

Le premier paquet cecf2247 et son contrôleur sans portée commune restent
préservés comme préparation non exécutée. Le paquet d’entrée retenu est
`~/.cache/rag3db-banc-notes/residence-nuit-12nov/gpu-01-sources-v2/`, aux deux postes.
SHA256 du plan : `a580c1711039436520dee35f3318fa605f10518189da4f3c4f731a76d401ace0`.
Contrôleur : `a29173c8ad1696e1e1c70460356c29049249ee9861ec7a688f296769693125ef`.
Entrée commune copiée de master24f3603cf, sans modification :
`confiner_validation.py` SHA `4f57d0e589f54d1e5733bcd01d793ce12b10e64ddd05220d22a9719084641c94`,
poste SHA `3bc31281db505837af7c6f7fa0ce208264bc63ca4e31dc5a8e026c9512de9dd6`.

Commande prévue, sans poste extérieur :

```text
python3 <paquet>/confiner_validation.py lancer --mode mesure --delai 1200
  --preuves <parent de preuves neuf>
  -- python3 <paquet>/mesurer_prive.py --plan <paquet>/plan.json --preuves <dossier neuf>
```

Le contrôleur vérifie le reçu vivant via la primitive commune, l’ascendance
poste réellement en mesure et le plafond hérité. Il reprend l’échéance du
superviseur (réserve de cinq secondes pour sa fin), sans recréer bus ni scope
sœur et sans relancer un budget de vingt minutes chez l’enfant. Les dix filets
CPU du contrôleur ont été rejoués après cette adaptation ; syntaxe verte.
Scripts, fournisseur, consommateur, BPK et tokeniseur sont listés et hachés
dans le plan. Variables du seul privé : GPU embedder gpu:0/Vulkan, ROCR0,
chauffe activée, relecteur/OCR vides, chemins 107m explicites et rayon8.

L’orchestration a relu protocole, groupes et empreintes, puis accordé l’entrée
après libération des builds. Relecture poste libre avant entrée ; Embarquement
confirme ne pas lancer de modèle privé pendant ce créneau et reporte ses e2e.
La source requalifiée4866 a une passe complète verte annoncée ; les copies345
restent immuables et RUST identiques pour le runtime concerné. Le plan et les
anciens rouges sont conservés. Aucun service partagé à arrêter ni modèle réel
chargé avant l’acquisition supervisée ci-dessus.

## Mesure exécutée : verte

Le créneau commun a été acquis le 12 novembre à 23:54:31 Europe/Paris.
La mesure réelle termine en 5,866447 secondes, verdict VERT sans résidu ;
les services partagés restent identiques et la VRAM revient au niveau initial.
Les [résultats et limites](03-resultats-residence-privee.md) rassemblent les
observations des deux activations, l’annulation, le maintien, la reprise et
la preuve de calcul sur PCI04. Aucun nouvel essai GPU n’est nécessaire pour
consolider cette observation. Le ticket commun reste ouvert pour son périmètre
plus large ; la branche Banc ne doit pas être fusionnée entière avec son ancien
runtime de préparation.
