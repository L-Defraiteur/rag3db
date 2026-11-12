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
