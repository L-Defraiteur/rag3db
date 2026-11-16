# COPYA : premier rouge du juge de métadonnées

Exécution autorisée par C7 sous allocation `C7-COPYA-9a-16nov`, pilote exact
`9a2cbf5449a7a4e823447ec9a4bb49f9b9a9bae6`, CLI927 ELF
`0e310a9cfe1c9e12534f3a579b3e781dcef7e9335e37d7e57349b16d053d5288`,
SDK59ac. Aucun rejeu après le premier refus. LIVE reste annulé.

Le poste réel et le scope commun validation-dvwp7t2a imposent 32G,
memory.high=max, swap0, cpu.max=500000/100000 et échéance monotonic
1190349.859759264. Compilation C++ rc0 en0.980727s ; selftest typé rc0
en0.003208s avant DB ; copie privée fidèle, inventaire initial du graphe,
cinq ensembles FTS et 469 entrées persistées contrôlés avant provider.
READY publié à1188707.480896897, maximum4166caractères, payloadSHA
98e221244872f5ce94b4ee0b84ec038a768febce7a7497711381207f34b29659.

ACK frais Embar : receiptSHA
327b6bd17c67d9a440d2502cdc52060d2364d242306c9f328c231f4dd093cb1a,
observed1188754.201466421, valid_until1189654.201466421. Identité réelle
granite-278m/768 dense seulement, cap12000, usage0/0 avant appel.
Writer privé PID3077217/start118875628, ELF exact ; profil réel et maps
SDK capturés. Aucun appel7970 ni opération de service effectués ici.

## Refus et cause établie

La réponse JSON brute contient ok=true, validation.accepted=true,
partial=false, signals=[] et **pendingCount=0**. `verify_meta()` lit
`pending_count` : get retourne None, donc le juge refuse la réponse.
Premier rouge immuable à1188885.336819958 :
`source.meta refused or partial; not a debt-zero proof`.

Il s'agit d'un défaut du juge de protocole, pas d'une preuve que le
SourceNode a refusé ou laissé une dette. Le driver refuse avant l'envoi
du shutdown ; le confineur constate des descendants laissés puis les
nettoie. Scope ROUGE/code1/restes[]/nettoyé=true ; autorité terminée.
Lecture /proc après arrêt : aucun owned vivant avec son start attendu,
bus privé absent. Ces constats de nettoyage ne qualifient aucune écriture.

La copie a pu être modifiée. Aucun SQL post-writer, aucune qualification
debt0/dimensions/indexREADY/graphaprès/FTSaprès ni comparaison finale de
la source n'ont été exécutés. Les 469 ne sont donc ni déclarés encore
présents ni déclarés acquittés. Aucun compteur de calcul/réutilisation
déduit. Source originale et LIVE n'ont pas été ciblés.

## Conservation et suite

PC : `~/.cache/rag3weaver-build/bascule-live-20261116/debt-copy-a-red-01`,
376 fichiers, manifesteSHA
373dccfa2a9550621fed8c89e0922ea3db56a9ff0666f7a0ed1c7528d398e687.
Le paquet conserve fichiers racine client/contrôle, logs bruts et scope
public ; il ne copie pas les données DB ni le jeton d'autorité. La copie
écrite, P/Q/S et anciens paquets restent conservés.

Avant tout correctif, le défaut est consigné dans le ticket existant.
Suite à arbitrer : témoin CPU du vrai verify_meta sur réponse publique
camelCase, vu rouge avant correction, puis refus stricts conservés ;
nouveau couple et revue, nouveau créneau/GO distinct avant toute autre
exécution. Ne pas corriger le produit ni réutiliser ce GO.

Conservation locale : les376 tailles/SHA ont toutes été recalculées sans
écart dans debt-copy-a-red-pc-local-01. Contrôle supplémentaire après
nettoyage, sans ouverture DB : inventaire fichiers et répertoires SOURCE
COPY02 strictement identiques, tous pins CLI/SDK/helpers inchangés, source
pilote propre exacte ; scan privé found=[] et12 UNKNOWN globaux non
attribués conservés. Ce contrôle n'est pas une validation des écritures.
Paquet debt-copy-a-post-cleanup-invariants-01,3 fichiers, manifesteSHA
8a26bfdd6aa4b801f12709bae9cd21c988e047afa9ebcae294b157eeb9036ca0.

C7 demande de proposer ensuite un audit RO de cette copie figée, sans
Source/embedding/writer supplémentaire : mêmes juges SQL/graphe/FTS/gen/
dimensions/debt, sous un futur GO distinct. Même favorable, il ne prouvera
pas le shutdown propre non observé ni une qualification de durabilité.
Embar réatteste après nettoyage usage0/0, identité provider inchangée et
327owned absents ; sa réservation est rendue, aucun nouveau grant acquis.

## Correction du juge préparée, sans nouvelle exécution native

Le vrai verify_meta a été vu rouge avec les392octets exacts de writer.stdout,
conservés dans writer-meta-response.json (commitdd6a953473 du témoin avant
fix). Le DTO SearchMeta de search.rs porte serde(rename_all="camelCase").
Le juge lit désormais exclusivement pendingCount entier0, refuse snake-only
et toute présence de l'alias pending_count, même concordant ; false/null ne
valent pas le nombre0. partial doit être explicitement false ; warnings et
signaux restent refusés. Aucun changement du produit ni des autres phases.
16fonctions CPU hermétiques passent en0.032s (10contrats+3logs+3meta), dont
la vraie réponse et variantes absente/snake/conflit/alias/type/refus. Aucun
compiler/DB/provider/processus réel lancé par ces tests. Le rouge natif et
son absence de shutdown demeurent inchangés.

## Proposition d'audit RO du résultat figé, non autorisée à l'exécution

Préparer un contrôleur séparé, aucune branche writer du pilote : nouvelle
destination de preuve, borne900 proposée après poste32G avec même enveloppe
outer/finalisation et premier refus STOP ; arbitrage C7 et RO du code requis.
P reste celui du résultat rouge, pas nouvelle copie ni reprise SourceNode.
CLI/provider ne sont jamais invoqués, pas compilation de helper : ELF typé
déjà compilé et son pin typed-before-db, SDK59/IDS et sources exacts à figer
dans un nouveau ledger avant toute DB. fd/exe/cwd/start et inventaire copie
avant ouverture ; profil RO pool1G/threads4 et états privés distincts.

Lectures proposées : audit.snapshot avec les mêmes469UUID puis comparaison
exacte à graph-before (mêmes exclusions explicites de vector/marker/claim
sur cohorte seulement) ; fts et comparaison exacte à fts-before ;
audit.finished avec SQLglobaldebt, dimensions768/finite/marker sur TOUS les
chunks, couverture469, claimsformat/writerunique et show_indexesREADY/loaded.
Inventaires fichier/dirs copie avant/après stricts, même .readers déjàprésent ;
sourceCOPY02/pins stricts après lectures ; quiescence et finalisation sous
même échéance. Aucun LOAD forcé, recovery, writer, purge, rescan, changement
de génération ou retry : si ouverture RO refuse l'état après cleanup, STOP
et documenter sans le réparer. Si indexREADY ou toute autre assertion refuse,
premierrouge et pas de contournement.

Un succès éventuel serait seulement un état logique/physique observé RO
après cleanup. Il ne transforme ni l'ancien scope ni l'absence de shutdown
propre en VERT et ne qualifie pas la durabilité, migration ou LIVE. Aucun
GO900 n'est acquis par cette proposition.

### Contrôleur RO matérialisé pour revue

`debt-copy-a/audit_result.py` implémente cette proposition séparément de la
branche writer. P/Q/S neufs : debt-audit-ro-result-v1,
debt-audit-ro-control-v1, debt-audit-ro-scopes-v1 sous la racine commune.
`--prepare EXACT` ne fait que copier les7sources et épingler timeout,
SDK/helpers et9références immuables du rouge ; pas d'ouverture DB. Le reader
existant add01058f3b0d6efd28d936c865b1ba9071c0e2eb85f559743bf0c52aeea6371
est réutilisé directement, sans build ni selftest natif ajouté.

Le futur `--launch EXACT --allocation REF` exigera destinations neuves et
source propre exacte. Poste réel puis timeout898+kill2 englobe compose,
common830, copie/intégrité, toutes lectures, nettoyage, hashes finaux et
publication. Chaque SQL90/FTS60/copie120 est borné au même END−15, pas de
reset. L'autorité32G/HIGHmax/swap0/quota<=8 et le bus privé sont contrôlés ;
TERMINE/ownedstartabsents/busabsent et deadline après hashes/prépublication
conditionnent le statut FROZEN_RESULT_RO_OBSERVED.

Avant la première ouverture : scan résultatfigé FD/exe/cwd/start, hash complet
de son bundle DB+FTS+sidecars ET COPY02, copie complète reflink obligatoire,
comparaison fichiers/modes/répertoires exacte, nouveau ledger et profil.
COPY02 n'est jamais ouvert en DB. Les références graph-before/fts-before
proviennent du rouge et sont épinglées à la préparation. La seule ouverture
native vise P/copy/code-principal.rag3db. readOnly/pool1G/threads4 du helper
préqualifié sont conservés ; aucun CLI, provider, Cargo, compilateur, recovery
ou LOAD n'est lancé. Reader générique réutilisé uniquement par les trois
juges explicitement listés.

Après lecture et encore au final : bundles sources strictement identiques,
copie existante octets/modes identiques, répertoires inchangés. Seul ajout
possible : code-principal.rag3db.readers vide/mode0644/SHAempty, exclusivement
sur nouvelle copie ; aucune suppression/modification d'un .readers existant
ni aucun WAL nouveau admis. Les UNKNOWN globaux sont conservés comme tels.
Premier écart immuable arrête le parcours ; nettoyage n'est pas attestant.

Trois fonctions CPU hermétiques passent en0.004s : exception.readers exacte,
refus WAL/mutation/suppression, contrôle AST des vrais raccords/commande de
copie complète avant Reader. Un premier test d'ordre utilisait ast.walk
(ordre en largeur) et a été corrigé pour comparer les numéros de ligne ;
aucune correction de comportement du pilote n'en découle. Ce ne sont pas
des tests natifs. Q/P/S ne sont pas encore créés ; préparation PC et revue
du couple exact précéderont tout éventuel GO900 distinct.
