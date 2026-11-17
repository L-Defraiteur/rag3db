# RO code et harnais POST/PRE/COLD

Avis **favorable statique** sur l'ensemble e9f037be2a, correctif POST local
0e4c296e5b conservé et raccord COLD ajouté. Dix-neuf fonctions écrites,
**NON JOUÉES par ED**, pas de résultat vert ni de paquetQv9 validé.
Même ticket distinct garde POST-writer, suite contrat07/RO38, preuves65eb
et anciens budgets clos immuables. Aucun native/DB/provider/recovery lancé.

## Code POST et persistance

Deux snapshots complets sont sauvegardés/fsyncés AVANT writer. Contexte
préwriter lie base privée, pilot, preparedSHA, nonce, END, CG, profil et SHA
des snapshots. La garde relit fichiers/contexte, les compare aux valeurs du
vrai driver et refuse substitution/autre cycle. END/firstred avant/après
collecte et publication/admission ; refus explicite des preuves absentes
ou mal formées, sans transformer l'absence en permission.

postwriter_inventory est distinct de stable_readers, dont les octets sont
inchangés. Vérification des preuves PID/start/exeSHA, mêmeprocess dans fermé/
publication/shutdown, ACK booléen true et rc entier0, CG/profil/maps SDK,
base et source. Rejeu des DEUX frames par Session existante, même nonce/run,
publicationSHA liée à demande shutdown et observation publique égale au RAW.
Ce sont des observations locales pinées, pas une preuve anti-forge mêmeUID.

Contrat physique : inventaires listés/typés, doublons/chemins protégés et
ambiguïtés déclarées refusés ; DB présente/mode inchangé/taille positive,
WAL/marque exacts absents après shutdown, tous autres fichiers stricts sauf
ajout .readers vide0644 initialement absent. Répertoires/UID stricts.
L'admission rend logical_integrity_qualified=false ; les juges bits/claims/
graphe/FTS/dette/source/quiescence restent obligatoires et séparés.
P/Q/S et manifeste sont cohérentsv9, ancienQ8 conservé ; Common/ARM/HB/final
et stable_readers ne sont pas élargis pour faire passer ce nouveau contrat.

## Raccord COLD après le writer

e9f remplace l'égalité brute finale after_reads==after_writer par le vrai
stable_readers(after_writer,after_reads), **DB principale incluse**. Sources
et répertoires restent comparés strictement avant cet appel. Cela autorise
uniquement l'ajout readonly vide prévu, pas un nouveau delta DB/FTS/auxiliaire.
Le validateur readonly partagé n'est pas modifié ; aucune ancienne innocence
de Qv8 ni reconstruction du before_writer RAM n'est déduite de ce raccord.

## Dix-neuf témoins relus sans exécution

QuinzePOST : vrai bloc ASTdriver/save et appel de la vraie garde compté
exactementunefois. Fixtures neutres positives et preuves complètes pour
ne pas masquer les mutations ACK/rc/identité/nonce/run/SHA/profil/maps/END/
firstred. Bool/int, UID de dirs, substitutionbaseline/contexte, doublon POST
sansWAL/marque, .readers et readonly antérieur conservé couvrent les réserves
ED précédentes. Sauter la garde fait échouer le compteur ; ignorer les
retraits/changements arbitraires échoue aux négatifs.

UnPRE : vrai ordre source assignment/save2inventaires/writer ; spy lit
présence/contenu/inode fsync au moment de l'appel. Les autres actions sont
explicitement exclues, aucune qualification du driver physique par ce seul test.
TroisCOLD : vrai bloc ASTafter_reads→avant sources-after, vrai readonly ;
positif nouveau.readers0, négatifs readers nonvide et DB modifiée. Les noms
des tests et leurs fixtures ne sont pas des attestations OS/SDK réelles.

## Rouges historiques relus

Trois fichiers par archive rehashés taille/SHA LOCAL+PC, zéro écart :

| Rouge exact | Manifeste |
| --- | --- |
| postwriter-causal-red-48a2-20261117-01 | 046c89cf87dbc61fd319f01ba024f47b48dc198a3f503cc29e53c6be2e5c1d64 |
| postwriter-cold-red-336c-20261117-01 | 0ece970a239988394f7ebe05952b0a8b066c957e8ce13ab88271026f17f4488f |

POST :1FAIL readonly refusé capturé→ASSERT,0ERROR/rc1/0.008s.
COLD :1FAIL égalité trop stricte capturée→ASSERT,0ERROR/rc1/0.007s.
Ces causalités ne sont ni des résultats natifs ni des verts de l'ensemble.

Avis transmis Arbre+C7 pour décision CPU19 exacte DISTINCTE, trois modules
POST/PRE/COLD sous borne59+kill1 si C7 l'admet. Aucun test/importCPU/proc,
DB/CLI/health/writer/service/Cargo/SDK/target joué ici. Après qualification,
source/paquet neufs et RO totale restent requis avant éventuel GO natif.
