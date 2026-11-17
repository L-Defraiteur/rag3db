# Trois bases : couple produit, retours et recette à arbitrer

Préparation C7b27e1b37c7, ticket parent complété AVANT cette suite en9a8b15aa9a.
Succède au plan12cfad sans réactiver aucun GO. **Lecture Git/documentation
uniquement** : aucun test, build, CLI, provider, scan de processus, service,
snapshot, copie de données ou ouverture DB exécuté pour cette préparation.
G reste propriétaire services/socket/7970/pose/retours ; Banc porte l'oracle.

## Acquis bornés et décision proposée

C7 retient Qv9 comme cycle observé sur UNE copie privée FIN, CLI927/SDK59,
preuves6e97b2ff8a6a8f4e4ca095ce3033fce10ceb8549 +doc965a2841706782d74aa7a4e8272445e20dfb6a56,
deux RO favorables R/ED41 (coquille SHA corrigée ED9cdab843eb). PRE persisté,
réponse publique/shutdownACK mêmePID/maps/rc0, POST physique puis graph/FTS/
bits/sources/COLD passés. Pas de durabilité crash ni LIVE ;11UNKNOWNglobaux,
computed/reusedUNKNOWN et anciens rouges65eb conservés. FIN après ancien rouge
ne reçoit aucune qualification rétroactive. Les deux autres bases n'ont pas
ces preuves par transfert et aucune des trois bascules n'est achevée.

Proposition à C7 : choisir le couple courant **source99fef92ce6 +SDK59ac** pour
une future qualification distincte, avant de fabriquer une recette trois bases.
927 reste une provenance qualifiée historique, pas un alias de master. Conserver
le SDK explicite actuel est une proposition fondée sur l'égalité des entrées
moteur, pas une nouvelle preuve ABI/runtime. Aucun nouveau build ni budget
accordé par ce choix documentaire. Un autre choix exige son audit exact.

## Fetch et audit reproductible de sources

Fetch origin/master effectué, HEAD distant observé
**99fef92ce6ee9a0007eaf99c7c3a6a3789061c57**. Cela ne garantit pas un futur HEAD.
L'[audit JSON](audit-master-99fef-vers-cli927-sdk59.json) conserve modes/OID et
SHA256 avant/après, empreintes des arbres, versions et limites. SHA256 du
fichier au gel : **51df30a6202c89bcf988e7bf974b025da03703e5b1aeaca0c5a2eaa2d1fcea34**.

Source CLI927941a67a5805450d49f1e04b39fdd542a6059b→master : **17 chemins hors
docs**,13 Rust et4 outils de poste. Aucun retrait ;12 modifications Rust et
un nouveau module de tests Vertex. Lecture des patches :

| Groupe | Chemins / effet à qualifier |
| --- | --- |
| Recherche | search.rs et generic_search_nodes.rs : départage stable score puis identité chemin/ligne/nom/contenu ; témoin dans backend.rs. Effet direct possible sur rangs oracle, pas équivalence de binaire. |
| Débits | embedder.rs, daemon/embeddings.rs, model_source/activation.rs, catalog/progress.rs +tests catalog.rs : adresse réellement prise et poste dans la clé ; provenance inconnue refusée, ancien fallback lu. Aucun appel provider à déduire d'une lecture de clé. |
| Vertex | gcp_auth.rs, model_source.rs, openai_llm.rs et vertex_refresh_tests.rs : capacité de token conservée, renouvellement401 avant flux. À inclure dans la qualification du jeu de features concerné, sans contact cloud implicite. |
| Fixtures | codex_threads.rs : écriture d'exécutables de test isolée du parent. C'est du cfg(test), sans prétendre que tout le delta Rust est test-only. |
| Passe/push | passe_epinglee.sh +test : pin59ac au lieu373 ; push_garde.sh +test_stack_headers.py : vrais headers range-diff paddés jugés. Leurs copies de cache ne sont pas réattestées ici. |

Cargo.toml/Cargo.lock et gitlink parseur sont identiques mode/OID entre927 et
master. Parseur embarqué : **bd133850f89d6b12a3ca2b2f6ce238785ed37e3a** ;
lock lucivy-core4.3.9/lucivy-fst0.1.1 identique.259 entrées templates et73
scripts du crate identiques, empreintes Git dans l'audit. REGLES7 est un
contrat d'admission à réattester, pas un fichier REGLES.md imaginaire.

Source SDK59ac9c86bc2b3529b590a7462ab7051e3ad26b8a→master : zéro delta pour
src/, extension/vector/, cmake/, third_party/ et CMakeLists.txt. Hors docs/crate,
**7 chemins diffèrent** : power_cut_test.cpp, les4 outils poste, linenoise.cpp
et son test shell. Ne pas étendre l'égalité du moteur à tout le dépôt ni à un
binaire shell nouvellement construit. Les preuves runtime SDK59 disponibles
restent celles de G/Qv9, aucun artefact posé relu ou chargé ici.

Pins historiques à rattacher aux futurs artefacts effectivement utilisés :
CLI0e310a9cfe1c9e12534f3a579b3e781dcef7e9335e37d7e57349b16d053d5288 ;
librag3db b7e9fc75a3d2de04c72680b0309e8bb1df9eaadb5211ebbacf39716c6d0a2101 ;
vector1669d6392ec8067fcf92ae2ccc972fd7cd3086629fb82d1f5d254e90d72d7561.
Un ELF du futur master aura son propre SHA, features et provenance avant usage.

## Outils : état source et qualification à ne pas transférer

Les scripts bascule cold-service-guard/migrate-closed-base/read-only-query/
snapshot-files sont ABSENTS du master99fef lu : copies sourceE9 et pins séparés
dans l'audit. Ce sont des outils documentaires de lot, pas du produit intégré.
Le vieux migrate-closed-base.py refuse live et épingle encore coldguard3ba491
alors que sa source E9 est82d65776613315f29d8e7753815899373c639b0b78d5138dfacfd815a8f2f04c.
Ne pas le lancer ni aligner le pin seul pour contourner ce refus.

| Outil | Réemploi admissible à proposer, pas exécuté |
| --- | --- |
| Common6439 / poste | Conserver signatures argv qualifiées sans -B devant Common, rôles Python-B exacts et borne après poste ; paquet byte-identique et garde fraîche avant nouvelle admission. |
| Contrôleur/acteur/health E9 | Porter les primitives de Qv9 qualifiées avec tous pins et preuve nonce/lease/identités ; nouveau P/Q/S et scopes, aucune importation d'ancien grant/END. |
| Reader typéPCadd01058 / IDs immutablesd0dd13e6 | Qualifiés contre SDK59 sur PC, pas nouvellement qualifiés sur master ; préflights source/ELF/BUILD_ID/maps/ISA propres au poste et format avant utilisation. |
| QueryPC48ae2328 / portable43cffb08 | Artefacts distincts ; ne jamais transporter le lecteur portable ISA refusé au PC. Corpus, framing, erreurs et compte complet à juger. |
| Scanner Banc d6cbd33b | WorkingTree927 +rlib927/rustc1.98.1 ; scanner≠git ls-files≠read_sources_report. Rattacher ou requalifier le vrai scanner du couple retenu, pas un substitut rg. |
| Oracle search_code | Sources/jeu/comparateur disponibles, lecteur courant99fef/SDK59 non qualifié par leur disponibilité. Pas d'ancien lanceur lib373/checkout/suppression. |
| Build CLI futur | Recette/pilote/target unique/features/JSON artifacts/ELF et guards à publier avant GO de qualification. Le nom historique preparer-binaire.sh n'est pas un chemin Git présent sous docs/procedures/scripts au master audité. G fournit sa recette exacte, aucun script supposé exécuté. |

## Retours : source courante, reçus historiques et fraîcheur manquante

G référence57fbd9aa56f4b7caeacb891388f3f10cbbb39d71,
[rapport04](../../optimiseur/16-11-2026/04-generateur-retour-pose.md), pose sourcee51b0d0c5bda791a62f39910a8872f43f7912468.
Source master99fef recoupée : instantanes.sh SHA
0cecea650a9db3dbe9dbe57159f63372339dd99cb35ef2b0d5909cb99eec9437 ;
générateurba27110ea947697aa432f84ed5c8e9be9d06878717d9c5dd2738cab10c9e8cd4 ;
helper V2e1fd96b25c1cc61b70507c59f7f203acb21936e5b676f68c74b88d216f923854.
L'alias docs/procedures/scripts/restore_snapshot.py est mode120000 : son blob
est un lien relatif ; SHA du helper résolu distinct conservé dans l'audit.
Le bundle posé par G contenait des fichiers réguliers755 ; pas de nouvelle
attestation des caches ~/.cache/rag3weaver-notes-A/return-generator-b8d7579797 ici.

Ces outils couvrent inventaire SHA snapshot, voisins vivants AU RETOUR, WAL né
après, préflight global de tous ensembles et conservation de l'état après.
Après début une erreur peut laisser un état partiel : pas de retour atomique
général inventé. Qualification historique15tests/deux postes n'est pas une
restauration actuelle autorisée. RET_v2 principal24eced05414acb1e505b49ff018519820cc48cbbb6597824bf7af47befda452b
était filesystem_fixture/cli_authorized=false ; snapshotdiag122933/e215 et
reçus/runtime16nov restent HISTORIQUES, pas gates actuels ni72h courants.

Pour chaque base, G devra produire avant toute écriture un nouveau reçu
source/config/bin/lib/service/snapshot/RET précis, inventaires complets et
identités liées, absence d'ouvreurs, durée de conservation **au moins72h après
l'opération concernée** et preuve du retour sur fixture dédiée selon schéma
courant. Ne pas écraser anciens retours ni considérer leurs dates comme une
nouvelle rétention. L'ancien wrapper instantanes-bascule.sh inclut une quatrième
base memoire-longue : il est HORS recette trois bases et n'est pas exécuté.

## Sources et oracle : trois qualifications distinctes

| Base | Poste / chemin canonique historique | Corpus privé de comparaison disponible, NONMATCHED |
| --- | --- | --- |
| code-principal | PC, ~/.local/share/rag3weaver/code-principal.rag3db | pc-principal-02,6949fichiers,SHA5911ae8f24a660c2c525095357571422c0601bc4dead535ce22e5a1aa24ae232 |
| code-lourd | PC, ~/.local/share/rag3weaver/code-lourd.rag3db | pc-lourd-02,7965fichiers,SHA8f87e531ee2fadd37290437227d9ddeddeb59aa73db37d5b346495712cd9d512 |
| code-rag3db-depot | portable, ~/.local/share/rag3weaver/code-rag3db-depot.rag3db | portable-02,9074fichiers,SHAaecbedb04fb69cfcfe364268645ef339904668548daf626743d8fe44f1bb9401 |

Ces freezes Banc ne sont ni snapshots DB ni oracle qualifié. Leurs parseurs
dans le CODE INDEXÉ (PCprincipal0c72f681, PClourd353d2c43, portablec660893)
ne sont pas le parseur bd133 embarqué dans CLI927. Ne pas aligner ces arbres.
Sources/configuration/règles doivent être réattestées à la future fenêtre G,
non-suivis et modifications locaux compris, sans lire/copier .env* ni .vault.

Références Banc : contratdf02f39a06c9949d285acd08747824f30cfa0297 et état
9bcac1558c2f8ef88d5982f2116523231b49dfd4,
[recette04](../../banc/16-11-2026/04-oracle-des-copies-finales.md), parité01§12,
limites05 et FIN07. Regrouper par base modèles/colonnes/formes/rangvisibles,
manifestes effectifs minified5000/code1MiB et WorkingTree réel. Les métadonnées
zéro de .env.example/YAML sont bornées au suffixeABS : pas preuve relpath,
visibilité estampillée ni MATCHED. Le contenu de ces fichiers reste non lu.

Comparer A finale à B′ NEUF depuis LE MÊME état source, REGLES7/non-suivis et
manifeste effectif ; B historique n'est qu'une évolution si le découpage change.
Trente cas/mêmes k/filtres/contributions, bm25 et vector effectifs séparément
puis combinés. BM25 RAW Lucivy relatif5e-3 sur mêmes contenus/IDs ; JAMAIS RRF
search_code. Vecteurs appariés contenu : cos>0.99999 ET maxabs<1e-4 ; identités,
rangs, manquants et pertinence jugés séparément. Aucun seuil inventé sur rouge.
Le départage stable nouveau99fef doit être couvert, pas hérité du CLI927.

## Recette séquencée à geler puis soumettre aux RO

1. **Choix produit** C7 : source99fef/SDK59 proposés, ou autre couple audité.
   Si master entrant modifie produit/locks/gitlinks/contrats/outils : STOP/audit
   de ce delta. Qualification du nouvel ELF/build features/SDK et lecteurs
   nécessaires sous recette distincte, aucune réservation prise ici.
2. **Paquets privés par base** : source/pilote/pins et P/Q/S/nonce neufs, listes
   complètes artefacts inclus/exclus, modèle/colonnes/dette/maxpayload inventoriés,
   scripts de G et oracle Banc rattachés. Provenance de chaque binaire par poste,
   BUILD_ID réellement lu et capture SDK/maps/profil de SON vrai processus.
3. **Fenêtre G distincte** : G décide hold/arrêt contrôlé et atteste services ET
   sockets sans réveil implicite7970. Management hors Common, consommateur
   nonce/identités/validité sous portée privée, pas de bus user réinjecté.
   Source/runtime/retour72h frais avant snapshot/copie/écriture. Aucun ancien
   gatefresh733,API13:37,backends[] ou hold ne vaut état actuel.
4. **Diagnostic froid sur nouvelle copie du snapshot G** : identités/ensemble
   files/dirs/UID/modes/SHA, schéma dynamique REL0/multiplicité, deux sens,
   FTSIDs↔offsets/générations complets/doublons refusés, inventaire modèles/
   colonnes/dette/payloads. Aucune commande officielle si un gate refuse.
   Purge NON NÉCESSAIRE seulement si preuve fraîche l'établit ; compact REPORTÉ.
   L'ancien422 sans corps reste NONATTRIBUÉ, pas de retrymaintain implicite.
5. **Qualification officielle copie distincte** : argv source lu `CLI migration
   versions --base COPIE`, puis `CLI migration generations --base COPIE`, jamais
   original ni ancien lanceur inactif. Versions lot2000 existant et bilan complet,
   generationbackend0/ranglu1. rc0 peut reporter une dette : arrêt à première
   dette/erreur, proposition de paiement déclaratif explicite, pas0parhypothèse.
   Tous modèles/colonnes, toutes dettes contemporaines, pas cohorte469 imposée
   à une source neuve. Acteur/policy fresh et usage typed0/lease avant tout contact.
6. **Cycle et POST sur cette copie** : PRE persisté/contextelié, réponsepublique
   nonce/camelstrict/pubfsync→shutdownACK/mêmePID/rc0, garde POST physique distincte,
   graphe UNE fois/FTS/bits/dette0/sources/quiet/COLD readonly/final ENDfirstred.
   Qv9 est une primitive qualifiée sur FIN, pas la qualification de ce nouvel état.
7. **Oracle Banc** sur copie finale et B′ matched : source identique admise,
   ancien/B′ et pertes nouvelles distingués, seuils/pertinence ci-dessus.
   Un statut NONMATCHED ou un signal absent maintient la base NON QUALIFIÉE.
8. **LIVE seulement sur GO exact ultérieur** : nouvelle fenêtre G/snapshot et
   RET frais selon base, même pipeline qualifié et budgets spécifiques, G seul
   réveille/réouvre. Tout index/repassage hébergé attend dernière ligne terminée
   puis quiescence ; client/busy seul insuffisant. Premier rouge arrête base
   et suivantes ; G conserve état après et n'applique qu'un retour autorisé.

Ordre proposé : PCprincipal puis PClourd si C7/G valident une fenêtre commune,
portable séparé ; aucune base suivante admise par réussite de la précédente.
Les caps exacts de chaque recette doivent être calculés avant GO selon nouveau
volume/dette/payloads et coût build. Repères historiques seulement : versions
principal464s hors paiement, repassage45–60min, B′~13min ; ni délai accordé ni
promesse pour lourd/portable. Pas de nouvel END ou budget dans ce document.

## Pièces manquantes explicites pour arbitrage

Couple produit/ELF futur et sa qualification ; recettes sources exactes de build
et des lecteurs ; état service/config/corpus courant par base ; snapshots/RET
72h frais et nouvelles preuves retour ; manifests matched admission oracle ;
caps et paquets scellés par base. Leur absence est une condition technique,
pas une demande routinière de permission Lucie. Deux RO sur la future recette
exacte avant chaque GO opérationnel ; aucun original/CLI/service/cleanup touché
par ce plan, tickets restent ouverts.
