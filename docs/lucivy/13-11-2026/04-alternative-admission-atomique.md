# Alternative C : admission atomique et garde empruntée (préparation)

Ticket unique : `docs/orchestration/tickets/2026-11-13-la-ci-refuse-la-reouverture-du-temoin-physique.md`.
Porteur Lucivy, RO ED/C7. Décision coût inscrite avant diff : fusion de B retenue.
Les preuves A/B et179+6 restent intactes ; aucun résultat ne leur est substitué.

Source C publiée `fa1d0273c468b997163f3cf9d5f56bae052f522a` dans lucivy,
branche neuve `cx-lucivy-barriere-admission-atomique`, dérivée de3b (tests core
conservés) ; production de B reste disponible sur brancheFBD inchangée.
**Aucun test, build ni mesure de C n’a été exécuté.**

## Mécanisme et invariants proposés

Un AtomicUsize contient bit CLOSING haut et nombre des envois dans les bits bas.
Admission par CAS AcqRel : incrément uniquement si bit ouvert ; fermeture par
fetch_or(CLOSING) sur le même mot. Le CAS perdu face à fermeture observe le bit
et refuse : aucune fenêtre check/incrément. Compteur saturé refusé sans wrap.

SendGuard emprunte Completion depuis ActorRef pendant send/try_send, sans clone
Arc. À fin, fetch_sub AcqRel ; tant que bit ouvert, ni mutex ni notification.
Si closing observé, mutex de rendez-vous puis notification Condvar. Le closer
prend ce même mutex avant test du compte et wait, pour éviter réveil perdu.
Chaque fin en phase closing notifie, pas seulement active0 : avec plus d’envois
engagés que de places du canal, des notifications intermédiaires permettent le
drainage qui débloque les suivants. Pas de polling ajouté.

Un envoi terminé avant fetch_or n’a pas à notifier ; son payload est déjà dans
la queue et sera drainé. Un envoi admis mais non terminé reste compté et doit
notifier en phaseclosing. Les modifications du compteur sont des RMW AcqRel ;
la lecture Acquire du compte0 et le protocoleflume préservent la visibilité.
Drainage messages horsmutex, attente de tous admis, drainfinal, reprisepanic,
pinned/HandlerGuard/on_start et completionaprèsDrop restent inchangés.
Un acteur ou Drop bloqué à jamais peut toujours bloquer la clôture, sous délai
externe de qualification ; pas de garantieinventée sur panicabort.

## Témoins et qualification candidate

Tous témoins précédents conservés. Le témoin avec garde figée utilise désormais
std::thread::scope pour respecter l’emprunt, sans ajouter de cloneArc au chemin.
Nouveaux juges : atomic_admission_keeps_count_when_closing et
closing_drains_more_admitted_senders_than_channel_capacity (trois envois
admis, capacité1 déjàpleine ; impose drainage de plusieurs arrivées).

Pilote `outils/qualifier-barriere-atomique.py` : copie du piloteplein relu9b,
seul SHA source remplacé parC. Proposition même900s/8G/HIGHmax/j4/commun,
lib entière (nouvel inventaire attendu, pas de compte ancien repeint), puis
sixcoreexacts. Arrêt premierFAILED/rcnonzero, sameEND/posthash, verdict final
subordonné au confineur et à quiescence. Couple à relire avant GO distinct.

## Comparaison A/C préparée, après correctness seulement

Pilote séparé `outils/mesurer-envois-ac.py`, ancienA663975a / nouveauCfa1d027.
Même harnais, hookpanic/rawpersistant et protocole que A/B ; labels et chemins
C distincts, matrice A/C/C/A96, locks identiques, mêmes4CPU coordonnés,
MESURE600 candidate neuve après poste exclusif. Aucun nouveau seuil inventé,
coût du DELTAglobal, biaispinneddiagnostics toujours déclaré. A/B brut/rapport
reste inchangé ; aucune comparaison A/C n’est encore une preuve exécutée.

Reste : ROED mécanisme CAS/fermeture/notifications/emprunt, GO correctness
exact puis, si vert, RO/GO mesureA/C distinct. Pas de merge, tag ou publication.
