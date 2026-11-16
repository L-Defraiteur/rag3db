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
