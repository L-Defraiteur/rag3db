# Contrat consommateur v2 : invocation depuis une entrée confirmée

**Version documentaire neuve, inactive.** Arbitrage C7 du ticket commun à
93c44be2d6, lu avant écriture ; ticket anglais complété73821fdec4 avant cette
note. La proposition961/note11 et ses vingt spécifications sont conservées
comme historique plus fort. Le présent changement de garantie est explicite,
pas une réussite rétroactive de P17–P20. Aucun code ou test modifié/exécuté.

## Garantie retenue par C7

Qualifier **l'invocation d'un programme déclaré et revu, lancé depuis une image
d'entrée effectivement confirmée**, avec copie de ce contenu, catalogue/profil
préautorisés, arguments/contexte, RAW de l'invocation et vrai wait du processus.
Le résultat porte sur cette invocation. Il ne signifie ni que l'image A est
restée inchangée pendant toute la vie du processus, ni que chacun des octets
sortis a été écrit par un PID déterminé.

Les programmes et sources sont déclarés et revus. Leurs enfants peuvent faire
partie du comportement normal des tests. Ce contrat ne promet pas de détecter
un programme arbitraire qui inventerait ses propres résultats. Les garanties
d'attribution des ressources et d'arrêt/reap du sous-arbre restent celles de
Common ; elles sont distinctes d'un relevé exhaustif de toutes les images.

Les futurs exec, générations et descendants après libération ne sont pas
couverts exhaustivement : **UNKNOWN**, sans passer pour satisfaits. Leur suivi
exhaustif n'est plus une condition implicite de livraison du lot anglais selon
l'arbitrage C7. Si un autre usage l'exige, il nécessite son propre lot/coût.
Aucun traceur persistant, seccomp, privilège ou sandbox modifiée n'est autorisé
pour contourner ce bornage.

## Faits nécessaires à chaque invocation racine

1. **Attendus indépendants** : source APP/pilote/Q, manifestes/locks/features/
   defaults, SDK attendu, programme et rôle, catalogue/ignored, argv/filtres,
   contexte d'environnement autorisé, plan/phase, allocation et END. B ne
   devient jamais attendu depuis le seul contenu découvert pour sauver la passe.
2. **Objet et copie** : catalogue et contenu sélectionné préautorisés ; copie
   stable préservée et empreintée ; lien à l'objet effectivement présenté à
   l'entrée. Un FD ferme certains changements de chemin, pas automatiquement
   une mutation de bytes. La stabilité jusqu'à confirmation doit être prouvée.
3. **Entrée confirmée** : identité réelle runner ET enfant séparée, PID/start/
   UID/cgroup, identité de l'image d'entrée et de sa copie, argv/contexte liés.
   Succès confirmé au vrai raccord, avant libération vers le corps des tests.
   Intention d'appel, FD, EOF de pipe ou simple Popen ne suffisent pas seuls.
   Le mécanisme concret appartient au producteur partagé et reste à livrer.
4. **Invocation** : RAW sur les sinks déclarés, code/signal du vrai wait et
   limites associés à cette occurrence et à ce processus. PID/start est figé
   avant reap ; disparition ou réutilisation ne permet pas de relire un autre
   processus comme s'il s'agissait du même. Le wait final n'est pas un rc d'A
   inaltérée après un éventuel A→B, et RAW n'est pas une attribution par octet.
5. **Publication et fin** : mêmes références catalogue/contenu/entrée/occurrence/
   phase/source/RAW/rc/END ; intégrité de la copie et des publications, première
   cause immuable et arrêt attribué/quiet. Copier/rehash/fsync/ACK compose le
   noyau CP ; publier ne produit pas le fait d'entrée. Rien n'est reconstitué
   depuis un résultat positif quand la confirmation a manqué.

Ces champs constituent une obligation consommateur conditionnelle, pas un
format de reçu runtime déjà disponible. Le futur producteur doit fournir la
preuve réelle, pas remplir ELF_PROVENANCE depuis les attendus ou les fixtures.
La surface API et les témoins partagés sont portés par Archi ; CP reste porteur
publication/ACK/Window ; TTY adapte le consommateur après revue autorisée.

## Catalogue, reconstruction et post

Un B différent après inventaire A exige un B déjà autorisé au plan, publié et
acquitté avec son catalogue AVANT l'action dépendante, sinon refus. Aucun
relisting opportuniste, nouvelle génération ajoutée depuis l'actual ou reset
END pour faire passer. A→A peut partager le contenu, pas l'identité de l'ancienne
invocation. Chaque racine attend sa propre confirmation d'entrée.

Un chemin T restauré après une entrée B ne change pas le fait capturé à l'entrée.
Une copie P=A n'atteste pas une entrée T=B. Après l'invocation, la copie et les
publications qui constituent la preuve restent intègres. Un nom T réaffecté
par une reconstruction autorisée ultérieure ne remplace pas la preuve de
l'occurrence précédente ; ne pas comparer seulement le chemin final global.

Le « post » de cette version désigne la fin/restitution et l'intégrité des
preuves de l'invocation. **Il ne prétend pas observer une image restée A** ni
couvrir chaque génération après la libération. Une observation ponctuelle
ultérieure ne sera pas transformée en continuité exhaustive. Le validateur
virtuel14 v1 n'est pas automatiquement un consommateur physique v2 : son
schéma pré/post doit être adapté et éprouvé au vrai raccord dans le futur lot.

## Table de portée des vingt spécifications historiques

Toutes les lignes sont des spécifications, pas des tests physiques acquis.
Leur état reste NON QUALIFIÉ tant que le producteur et le raccord n'ont pas
leurs preuves propres. Les14tests hermétiques n'en changent pas cet état.

| Référence | Portée v2 retenue / limite |
|---|---|
| P01 | Entrée racine A confirmée + catalogue/copie A + invocation/RAW/wait liés ; reste à prouver |
| P02 | Image d'entrée étrangère au plan au même chemin/noms : refus |
| P03 | B à l'entrée puis chemin restauré A : conserver le fait B et refuser si B non autorisé |
| P04 | Rebuild B après A sans catalogue/ACK préautorisé : refus avant action dépendante |
| P05 | B préautorisé au plan, catalogue/ACK antérieurs et entrée B confirmée : admissible conditionnellement, pas d'attendu promu depuis B observé |
| P06 | A→A avant une autre invocation : contenu équivalent possible, nouvelle confirmation/identité d'invocation requise |
| P07 | Copie A et image d'entrée B : refus ; ACK d'A ne prouve pas B |
| P08 | Identité runner/enfant incohérente ou PID réutilisé à la frontière : refus, aucun signal à un PID non attribué |
| P09 | Entrée non confirmée malgré logs positifs : HOLD/refus, pas de succès depuis un sondage manqué |
| P10 | Copie/catalogue/RAW de preuve modifié après publication : refus ; ne pas confondre avec le chemin T réaffecté après occurrence |
| P11 | Mauvaise invocation/phase/nonce/allocation/argv : refus de mélange |
| P12 | Référence héritée sans ACK, doublon ou séquence hors plan : refus par noyau commun |
| P13 | Temps expiré, réserve indisponible ou mauvais domaine d'horloge : refus sans budget neuf |
| P14 | Première cause pendant entrée/publication/invocation : rouge immuable et arrêt/reap sous END |
| P15 | Résultats/ordre/filets/noms/ignored non liés au catalogue de la racine : refus ; pas ANYrésumé |
| P16 | F8 supplémentaire/dupliqué malgré fin8/8 : refus du jeu ; conserve les précontrôles nommés |
| P17 | Futurs exec après entrée : couverture exhaustive UNKNOWN, pas condition implicite du lot anglais, pas satisfait |
| P18 | Descendants/images futurs : couverture exhaustive UNKNOWN ; Common garde attribution/arrêt des ressources sans se déclarer trace exhaustive |
| P19 | Tentatives échouées non exhaustivement comptées : UNKNOWN ; jamais B « exécutée » sans succès confirmé à l'entrée. Aucun comptage d'échecs déduit d'une absence de succès |
| P20 | Wait/RAW de l'invocation recevables au périmètre décidé ; pas de preuve qu'A est restée l'image ni de rc d'A après remplacement. Exhaustivité ultérieure non satisfaite |

La portée retenue pour P01–P16 concerne les racines déclarées ; elle n'étend
pas ces garanties aux auxiliaires non observés. Neuf positions Rust proposées
restent une projection de source, pas neuf processus/ni toute la descendance.
Les autres racines, auxiliaires et phases doivent être déclarés ou rester non
qualifiés. Une racine non couverte ne reçoit pas le vert d'une autre.

## Canon et responsabilités

C7 accepte les quatre sites directs e2e et le raccord Cargo runner comme
**surface transitive proposée** ; aucun de ces appelants n'est modifié ici.
La réservation avec Recherche précède toute modification. Le blob canon2150
inchangé ne sera jamais annoncé comme preuve d'un parcours transitif inchangé.
Les variables propagées, dont LD ajusté par Cargo, argv et profils de chaque
phase restent à déclarer et à qualifier. Aucun wrapper PATH/canon local ad hoc.

Archi prépare le contrat/API minimale, le mécanisme précis de confirmation et
les témoins partagés AVANT implantation/exécution ; observations de processus
modélisées ne qualifient pas l'hôte. TTY apporte la présente portée et relit
les preuves pour l'invocation. CP publie les faits, sans second producteur ni
boucle de publication métier TTY. Les appelants et un futur petit pilote réel
nécessiteront leurs propres sources/pins/budgets/GO et régressions.

## SDK, horloges, ressources et coût

Le SDK dynamique effectivement chargé reste une attestation distincte au PID
et au point réellement observés. Image d'entrée confirmée ne prouve ni les libs
chargées ensuite ni leur continuité. Pas de copie d'un succès229b, pas de lecture
d'environnement complet ou de secrets pour remplir cette absence.

Common garde ses plafonds et son END original. Entrée/copie/canon/fin/arrêt/
publication/reap tiennent dans le même budget, chaque étape MIN(reste frais,cap),
aucun reset après ACK. CLOCK_MONOTONIC@LOCAL et @PC ne sont pas numériquement
comparés. Les leases modèles PC, tunnels/SQL LOCAL, release et raccords de
publication gardent leurs propres admissions liées à cette invocation/phase.
Aucun service ni pouvoir de prolonger END ne découle d'une copie ou d'un ACK.

Les coûts/effets sur les délais et le contexte d'exécution sont UNKNOWN. Ils
seront annoncés et mesurés sous futurs GO dédiés, pas réputés nuls. Si le vrai
handoff, la stabilité ou la portée d'un appelant ne sont pas établis : STOP/HOLD,
sans relâcher un garde pour faire passer cette version.

## État exact après publication

Ceci est la version v2 DOCUMENTAIRE choisie par C7, pas une API livrée ni un
producteur actif. Les vingt spécifications961 ne sont pas vingt tests verts ;
P17–P20 ne sont pas satisfaits par l'image d'entrée. Le ticket commun et le
parent anglais restent OPEN jusqu'à qualification réelle au périmètre accepté.

ELF_PROVENANCE demeure vide ; tous les Q existants restent INACTIFS, sans
repin/activation. Aucun code producteur/consommateur, CPU, test, Cargo/no-run/
list/native/traceur/health/PG/service/slot/target/SDK n'a été exécuté ou modifié.
Le dernier résultat reste14témoins hermétiquesfe01/preuves4a6cc, hors garantie
physique v2. Suite : RO de portée et alignement avec API/témoins Archi ; tout
changement de code et chaque exécution nécessitent leur cadrage/GO distincts.
