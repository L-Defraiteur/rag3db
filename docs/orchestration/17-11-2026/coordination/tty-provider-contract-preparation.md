# Contrat inactif des providers du futur lot TTY

Préparation seule, suivi C7 `5b6ade163c5b7fdfd6591df8e3408a45b2fcec70`,
[ticket TTY existant](../../../tty/tickets/2026-11-16-les-commandes-et-options-du-terminal-en-anglais.md).
Le futur canon LOCAL2150 puis scope TTY3600 sont proposés, sans GO3600,
acteur armé, ACK frais, grant, réservation, santé ou création de tunnel actuels.
Construire un Q désarmé ne dépend pas d'une preuve runtime encore interdite.
Ce document n'autorise aucun contact et n'atteste aucune disponibilité présente.

## Sources communes à composer

- Acteur `grant_watch_actor.py` et test : source
  `4f54eef8af693c17e5401d1adc9d4c9062b315d3`, même dossier que ce contrat.
  SHA acteur `7d7db5a0c7d9ac762eed74ade2d0b9ee256b8499d0029ae970feac916de76612`.
  Qualification propre :20/20 CPU hermétiques, aucun processus réel lancé.
- Contrôleur/supervision et raccord cycle : préparation Arbre
  `2dccc070f1564d8ca9d22a59fcb2b73222f415e5`, code84a61e6934,
  124 fonctions CPU vertes rapportées par Arbre, pas rejouées par Embar.
  Ce couple est une référence du mécanisme, pas une autorisation de transposer
  la politique cycle vers TTY. RO totale du nouveau couple encore nécessaire.
- Politique globale `global_services_owner.py` : source
  `443b5a039cc2a3ff11705b0ebaa5bfa6577d6b0e`,
  [fichier](../../../recherche/16-11-2026/fts-generation-global-r2/global_services_owner.py),
  SHA `9d308b5bd2d1eb44dca02da8d02f8cce4a2a7717e57a246301b468c142bf6b8d`.
- Transport commun de cette même source :
  [provider_health_transport.py](../../16-11-2026/coordination/provider_health_transport.py),
  SHA `edf45f73bf97d21f2a0caab4d6e07a7d01f807fba8310c9006f442159e2bc2a6`.
  Worker GET /sante uniquement PC7878/7879, budget total5s avec kill/reap bornés,
  preuve brute bornée avant refus, statut200 et neuf identités typées strictes.

La politique TTY devra être publiée, pinée et revue avec son pilote avant GO.
Réutiliser acteur/contrôleur/transport ; aucun troisième client HTTP ou acteur
ad hoc, aucun appel de modèle pour reconnaître un service. L'acteur commun ne
fournit ni profil TTY ni réservation implicite. PG est porté par G, pas par un
ping SQL ajouté à l'owner modèles.

## Ressources et responsabilités par phase

| Ressource client | Cible attendue | Responsable de la preuve future | Phase autorisée |
|---|---|---|---|
| LOCAL127.0.0.1:7979 | PC127.0.0.1:7878 granite-278m/768, dense | G: topologie/tunnel ; Embar: identité/usage modèle sur PC | Phases TTY explicitement déclarées nécessitant ce modèle |
| LOCAL127.0.0.1:7980 | PC127.0.0.1:7879 bge-m3/1024, dual+sparse | G: topologie/tunnel ; Embar: identité/usage modèle sur PC | Phases TTY explicitement déclarées nécessitant ce modèle |
| PGtest5433 | Bootstrap et bases de test distincts selon le pilote | G et pilote TTY | Phases SQL/test déclarées, pas de déduction depuis une seule base bootstrap |
| Fake17990 | Fixture privée du futur pilote | TTY, après gardes | Témoins déclarés seulement ; aucun service C7 ouvert maintenant |

La topologie LOCAL→PC n'est pas prouvée par un GET local ni par le seul numéro
du port ; G doit fournir son attestation liée à l'allocation. Aucun tunnel
créé ou modifié dans cette préparation. Le transport commun est exécuté côté
PC sur ses adresses fixes ; on ne le détourne pas vers LOCAL7979/7980.
Une phase sans besoin de provider n'acquiert pas de permission d'appel.
TTY fixe avant GO ses phases, leurs appels et leur durée maximale, avec G pour
PG/topologie. La preuve G de parsing seule ne qualifie pas de futures lectures
PG. Un outil PG réel non qualifié reste HOLD ; aucun identifiant/OID inventé.

## Attendus non secrets figés avant toute admission

Le Q préparatoire doit distinguer configuration attendue et preuves observées.
Il contient les sources APP/pilote exactes, hashes de tous fichiers, profil de
ressources/phases, références C7/Embar/G, allocation, domaine d'horloge,
nonce du futur READY et hash READY calculé après publication complète. Les
identités attendues sont approuvées avant observation ; aucun PID ni modèle
observé n'est promu silencieusement en attendu.

Pour chaque provider, neuf champs exacts : `service`, `modele`, `dim`,
`factice`, `dual`, `sparse`, `precision`, `executable`, `max_chars_per_text`.
Attendus modèles : service rag3weaver-embeddings ; granite-278m/768,
factice=false/dual=false/sparse=false ; bge-m3/1024,
factice=false/dual=true/sparse=true ; precision Flex32. Chemin déclaré suffixé
et capacité sont figés dans le profil approuvé, pas découverts pour accepter.
Ajouter l'identité OS attendue `pid`, `uid`, `start`, `exe`, `exe_sha256`,
contrôlée avant/après observation. Distinguer executable déclaré par santé et
chemin OS réel. Les empreintes historiques ne sont pas des relevés frais.

Admission santé : HTTP200, types exacts, usage.baux=0 et usage.en_cours=0
(entiers, pas booléens), raw préservé borné, PID/start/UID/exe/SHA identiques.
Usage0 n'est ni preuve de zéro RPC historique ni réservation à lui seul.
Les payloads de chaque phase doivent être bornés par la capacité attendue ;
pas de troncature client silencieuse ni de modèle de remplacement. TTY doit
fixer et vérifier les limites de caractères de ses vrais chemins clients.

## Admission future sous GO distinct et acteur réellement vivant

Avant tout préflight/ledger/action de la passe réelle, le contrôleur lance
l'acteur versionné sous l'enveloppe originale autorisée, puis exige ARMED émis
dans sa première boucle contrôlée : PID/start/UID/cgroup/exeSHA/argv/scriptSHA,
configSHA/nonces/APP-pilote/allocation liés, identité OS relue indépendamment,
poll vivant et heartbeat frais (borne10s). Un fichier seul ou un message de
session disponible ne constitue pas ARM. Disparition, refus, sortie0 inattendue
ou heartbeat périmé donnent premierrouge immuable immédiat.

Les premiers contacts ne viennent qu'après READY complet fsync puis link
exclusif, nonce/hash exacts, autorité active et fenêtre d'admission validées.
Le helper strict revérifie avant GET et immédiatement avant link du grant.
La marge30 du helper cycle est un précédent à conserver pour son raccord,
pas une lease230s suffisante à une passe TTY3600. TTY doit publier les bornes
propres à son profil, sans allonger les fenêtres d'admission communes.

La lease doit couvrir la totalité de la phase d'utilisation autorisée et son
rendu/quiescence, pas seulement l'attente d'un ACK90s. Chaque action doit
encore tenir dans sa lease et END original ; sinon refus avant action.
Admission ponctuelle et lease d'exécution sont distinctes. Après admission
valide, ne pas réappliquer artificiellement la fraîcheur READY initiale pour
prolonger/recréer la lease. Aucune nouvelle horloge estimée : timestamps PC
sur CLOCK_MONOTONIC@luciepc ; timestamps LOCAL dans leur domaine propre,
aucune comparaison numérique directe entre postes. La correspondance des
phases/autorités/budgets doit être matérialisée et revue avant GO.

Après grant, l'acteur commun reste vivant sans nouveau GET jusqu'au release
DRIVER_FINISHED authentifié du contrôleur, dans END original. Rendu explicite
après quiescence attribuée : résultat terminal, PID/start/grantSHA/allocation,
preuve de cleanup et firstred conservés. Aucun arrêt des connexions baseline,
restart, warmup, changement modèle/SDK/7970 ou nouveau TTL depuis l'armement.
Une phase autorisant des embeddings utilise sa politique globale explicite :
le flag no_embedding_calls_authorized=true du cycle ne s'y transpose pas.

## Avant le futur GO

TTY matérialise phases/profil, APP/pilote/Q et hooks de politique ; G matérialise
la topologie et qualification PG réellement nécessaires ; Embar vérifie le
raccord modèle piné. Fakes publiés avant tests, rouge avant fix, puis revue
totale du couple et des pins. Le GO canon LOCAL puis GO TTY restent distincts.
Aucune autorité, identité fraîche, lease runtime, ACK ou grant valable ne peut
être fabriqué dans le Q inactif pour faire passer cette préparation.
