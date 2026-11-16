# Canonique v2 verte, seconde lib directe rouge

APP **734087a4e99d54331aac2f6321df9ef6da3b6d9c**, PILOTE
**f7ef1b0153dc5c337a9a9e3cb2e234c96a84e789** figés. GO conditionnel C7 appliqué
aprèsACK frais Embar077 (BGE/PG rendus, health200,baux0/en_cours0). Pas de
nouvelle autorisation implicite après ce rouge. [Parent mémoire ouvert](../tickets/2026-11-15-un-temoin-de-codex-rougit-sur-text-file-busy-sous-la-charge.md),
[fils socket](../tickets/2026-11-16-le-profil-tmp-prive-depasse-la-longueur-des-sockets-des-temoins-du-pont.md),
[cas antérieur identique montage ELF](../../arbre-principal/tickets/2026-11-12-migrer-les-ancres-de-code-vers-le-contenu-partage.md).

## Ce qui est vert et ce qui ne l'est pas

Le résumé **produit par la passe canonique complète** finit VERT, sans assemblage :

| Étape v2 | Résultat |
|---|---|
| codex_threads ciblé | 6/0/0,0,31s |
| F8 | 8/8 |
| Preuve | rag3weaver revérifié |
| Première lib entière large | 2596/0/72,0filtré,81,60s |
| NAV | 45/0/0,79,45s |

Les trois témoins sockets rougesv1 passent dans cette première lib v2. Les72ignorés
restent NON JOUÉS ; aucune addition de passages comme nouveaux juges. Aucun e2e
hors canonSUITES=- : pas de code_sync requis par le diff tests de ce lot.

Le résumé canonique est conservé intact, SHA
**0dddc68c959407fc97c88f024b1911fa9dcd1c718a738175b8f9f3d2e99076ca**.

La **seconde lib directe**, sans Cargo/F8/NAV supplémentaires, exécute le MÊME
ELF que la première : copie+ledger publiés avant cet appel, SHA
**6f12273abeb4a957dd5abdcd0f518ee673607146eefbe2c2938769d9bfdf8e74**,
484292712octets, identique également à l'ELF rougev1. Elle est ROUGE : premier
FAILED `code_tools::cwd_tests::cd_ne_ment_pas_sur_ou_on_est`. Pas de résumé final,
pas de compte complet ni de rc du binaire acquis avant STOP ; verdict composé RED.

Watcher exact sur second-lib.log : device55/inode166176650,49067octets,
SHA **94b97dc54fb5fa79a37f572217c9992c16a5c7362ece69e9e8a529d3a8c237bc**.
Préfixe exact archivé/hash relu ; first-red immuable1182622.79943142,
soit3047,120857327s depuis INITIAL. La canonique reste verte, la recette composée
reste rouge ; aucun repaint, retry ni nouvelle invocation après FAILED.

## Profil, provenance et temps

INITIAL original1179575.678574093 ; END original1186775.678574093. Admission
après vrai poste1182204.599350739, durée4568s, reçuvalidation-ozbiuey5 avec
END1186772.644046987<=original. Pas de reset7200 : compilationv1, rougev1,
diagnostic, corrections et pauses restent inclus dans cette même fenêtre.
32GiB/HIGHmax/swap0/cpu.max500000/100000 ; magasin SDK59ac explicite et SOhash
identiques, copie canonSHA2150. Source/pilote/gitlink propres recontrôlés.

TMP **/home/lucied/.cache/rag3weaver-build/m-0fdabc6d31**, btrfs/device55,
inode165798746/UID1000/0700, gardeUTF8pirechemin89octets. Identité relue au
relevé indépendant. Deux états frères sousP. Tsession existant source734,
marker/provenance/reçuTERMINE/archivesmanifest904/ELF/8647fichiersFP hashés et
sonde buildrsRO pré/aprèsposte capturés avant assertions, sanspurge/import.
Les empreintes évoluant ensuite par compilation légitime ne sont pas présentées
comme des invariants de toute la passe. Watchers actifs ED byteidentiques.

ScopeROUGEcode1 (« descendants laissés après la commande »), restes[],nettoyé,
autoritéTERMINE ; retour extérieur1. Le driver quitte au premier rouge, le
confineur arrête les descendants. Scan indépendant1182769.162588685 : found[],
owned_alive[],owned_unknown[],11UNKNOWNglobaux opaques conservés. Bus et cgroup
absents ; T5658160728octets et TMP conservés, poste rendu. Aucun pic mémoire ni
zéroGPU inventé : plafond prouvé, statistiques de charge globales non attribuées.

GET/sante après STOP :7878granite768dense et7879bge1024dual+sparse,HTTP200,
exe1794771361/Flex32/nonfactice inchangés, baux0/en_cours0 tousdeux. BGE/PG
rendus, aucun futur appel de ce run. Pas de SQL/reset ; pg_isready absent sur
lePC, donc aucune preuve supplémentaire de readinessPG revendiquée. Ni restart
ni API7970 ni base de Lucie utilisée par le pilote.

## Preuves et diagnostic RO

**35fichiers** PC/local réhachés taille+SHA sansécart :
[manifeste](script-fixtures-red-v2-manifest.json) SHA
**c804f26134bde1de2ced2b1ce8ad601dc2e829e854910313f47c998c3928d42c**.
Racines ~/.cache/rag3weaver-build/cx-proofs-memoire-script-fixtures-red-v2 et
suffixe-local, {client,controller,supervisor}. ELFcopié ; profils sans jeton ;
autorité privée exclue. Les preuves et rougesv1 restent conservés séparément.

Sourcecode_tools::cwd_tests::ici lit std::env::var("CARGO_MANIFEST_DIR").unwrap().
Le profil explicite du lancement DIRECT ne contient pas cette variable, alors
que cargo test la fournit à ses tests. C'est un défaut de montage du pilote
identifié en RO, déjà décrit dans le ticket ancres Arbre (section libv2rouge),
pas un défaut produit attribué. Le détail de panic capturé n'a pas été imprimé
avant le STOP : pas de message NotPresent artificiellement prêté à ce run.

Ticket parent enrichi **bd6095f406 avant proposition**. Proposition seulement à
C7 : variable runtimecrate explicite pour la seconde lib, nouveau pilote et
nouvelle provenance reçuv2/cache/ELF, sous mêmeEND et reste après poste ; aucun
rejeu/édition du pilote après le job, aucune reprise spontanée et aucun GOmaster.
