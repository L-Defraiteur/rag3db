# VERT ciblé réel : les deux setups appliquent le Sandbox avant tâche

APP **7425375d66a64b0782672225c59516f048682b00** ; pilote exécuté **01c3ae28e8218262ea545513d872f8f3370def0f**,
Qr3/config a1249cf2b7232337c5822f932fa4b56662d7a995c8a3b713d2abc94c1836e357.
GO exact C7-ARCHI-BAC-GREEN-01C3-16nov795, après revues totales code/CLI/pin
Mémoire et Banc. Le présent rapport et son commit documentaire ne modifient pas
le pin exécuté. Source/pilote/anciens paquets/rouges conservés.

## Résultat et compte exact

Deux bâtis réels Cargo JSON verts : lib36.88s et cible e2e31.52s,
no-default/code,daemon,rag3db-native,locked/offline/no-run. Deux ELF uniques,
features exactes et préfixe source742 mesuré dans leurs octets. Quatre inventaires
--list/--ignored ont précédé le ledger ; bons noms présents et non ignorés.

| juge distinct | parent | enfant même ELF/même nom | code |
|---|---|---|---|
| daemon_host::teammate_tests::task_fixture_uses_the_declared_private_sandbox_before_add |1/0/0|1/0/0|0|
| e2e_equipier::documentation_fixture_uses_the_declared_private_sandbox_before_add |1/0/0|1/0/0|0|

**Deux juges distincts, quatre invocations brutes**. Quatre exécutions
 d'inventaire non juges ; aucun témoin ignoré, aucun zéro-test, aucun retry.
Chaque enfant a attesté le manifeste réellement installé et son contenu dans HEAD
avant tâche : égalités déclaration/manifeste/HEAD, commands=approval, extra_read
Q privé, cible `{repo_target}`, TMP `{task_dir}/tmp`, jobs1, offline=true.
HEAD lib de6f20b3162cfa68ddfac96c8d8911ab00782aa6 ; HEAD e2e
9b1fa17fc9de759106fdd0c2953f22060d62738c. Les TempDir réussis sont normalement
détruits ; les stdout positifs sont conservés, aucune fixture verte reconstruite.

## Ledger et copies publiés AVANT runtime

Ledger b2ca5157dcdbcae27efb4c7edd7cb2c233322cec9095aa76f5aa71bbb6826d23.

| ELF | octets | SHA256 | offset du préfixe source |
|---|---:|---|---:|
| lib |263650384|f06ff55278e5ac55718088cec21676a21476f5a03b3a27e15111cf8e85921578|9698111|
| e2e |237785432|63b73273234397aa6dd6ae16d57087bade0fea8deb703c895d248fd60ff7aaa4|14659846|

Copies PC et portable réhashées avant publication. Séquence monotone réelle :
publication1201365.469081019 < marker1201369.739034006 < positiflib1201374.119015559
< positife2e1201378.543581242. Pauses et copies dans la même enveloppe initiale,
aucun reset. Reçus/copies avant témoins sous cx-proofs-doc-bac-green-before-witnesses-r1
sur PC et suffixe -local sur portable.

## Bornes, garde et quiescence

Vrai watchdog TERM780 + KILL15 lancé AVANT le worker --arm, tous précontrôles,
log/fsync d'allocation et file poste. Timestamp **observé dans le worker après
armement**1201209.606527503 ; soft1201989.606527503/hard1202004.606527503 sont des
majorants conservateurs, le timeout OS antérieur peut arrêter plus tôt.
Portée commune600s après poste : start1201221.786952387/END1201821.786952387.
Driver terminé1201382.776415216 ; finalstart1201382.867397639, capfinal120 donc
ENDfinal1201502.867397639. Résultat composé GREEN_SETUP_TARGETED_ONLY, elapsed
**177.93611381785013s enregistré** ; contrôle après écriture/flush/fsync passé,
même verdict dans fichier/retour, parent ultime rc0. L'elapsed enregistré n'est
pas présenté comme horodatage exact du dernier I/O.

Scope VERT/code0/erreurnull/restes[]/nettoye, autorité TERMINE et bus absent.
Plafonds effectifs memory.max34359738368, memory.high=max, cpu.max500000/100000,
POSTE_FILS5/CargoRayon5, enfant j1. Même T dev55/inode164550643, propriétaire
canonique742 ; garde commune SAMEOWNER code0/silencieuse/inventaire avant-après
strictement identique. Cache223 terminé, ancien cache horsT/archives physiques
1bd/049/223/EA et SDK59 réattestés. Stockage alloué session8827813888B,
runtime0B, ensemble8827813888B <32Gio. Aucun pic mémoire inventé.

Contrôle indépendant APRÈS retour à1201423.276 environ, conservé séparément
sans extension de délai :132observations attribuées, alive[]/unknown_owned[],
scan found[], contrôleurs0, cgroup et bus absents. Onze UNKNOWN globaux non
attribués consignés, ni zéro ni absence GPU déduite. Ce contrôle après retour
ne se substitue pas au final borné déjà passé.

## Archives et portée

**72 fichiers**, [manifeste](doc-bac-green-final-r1-manifest.json) SHA256
fdafa95b335b57fd2066738995d3edbb3de7a5b65ca3ef632832f1e827e06ef3,
réhash personnel PC et portable :72tailles/SHA identiques, zéro écart.
PC ~/.cache/rag3weaver-build/cx-proofs-doc-bac-green-final-r1 ; portable suffixe
-local. Tar42387aedc71bf2a4107dc096c4b7a58957dc9f23d54e5a8a6df741fdb5c6a783.
Client/contrôleur/superviseur public et deux ELF conservés ; autorité privée et
jeton exclus. Archivage et duplication postérieurs ne sont pas une extension
du budget de qualification. Anciennes preuves rouges223/72338/480604 restent
strictement rouges, y compris les anciens défauts du pilote et leurs journaux.

VERT du **bac ciblé seulement** : pas de Vertex réel, D6, F8 global, lib entière,
NAV, PG, provider, service7970, pose SDK, master ou retrait. Les sources et
ressources sont gardées. Le [ticket existant](../tickets/2026-11-16-le-temoin-documentaire-vertex-attend-un-accord-sans-superviseur.md)
reste OUVERT pour la boucle réelle et qualification complète/pas-va-au-bout.
Demande RO bornée de ces preuves et arbitrage de la suite à C7 ; aucun GO ancien
ou élargissement n'est déduit de ces deux positifs.
