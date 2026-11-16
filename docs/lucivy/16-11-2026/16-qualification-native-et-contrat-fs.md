# Natifs HOLD ; contrat FS et quatre rouges à préparer avant code substantiel

Ticket spécifique créé AVANT correction :
`docs/lucivy/tickets/2026-11-16-l-admission-du-corpus-fs-ne-garantit-pas-la-vue-consommee.md`.
Lié au père profil-core/diagnostics nonisolés ET au suivi indexed-false.
Les neuf juges mémoire verts1ee5 ne sont ni rejoués ni requalifiés FS.

Préparation **code+preuves archivées seulement** : aucun Cargo, ELF ouvert ou
exécuté, procscan, contact corpus (même metadata), service, modification Rust,
main/tag. Les pins ci-dessous sont historiques, pas un nouveau contrôle PC.

## Deux parcours, deux jeux, deux panels

[Pins et profils archivés](preuves/native-preparation-1ee5/SHA256SUMS), source1ee5.
Les deux ELF ont été compilés lors du GOmémoire en opt_level3/debuginfo0,
debug_assertionsfalse/overflow_checksfalse/testtrue, features default+mmap :

| Parcours | ELF dernier attesté | Entrée exacte |
| --- | --- | --- |
| Derived | 78fef16a51b354336989c4031f1032f14cd4883fab1ffbcc0762a382f7e17f17 | derived_in_ram_answers_like_the_files |
| Ground-truth | 917928a151c81a22324ca93eeb007a9aa17338781c7d2131c407f8d4fd5f14d4 | v3_ground_truth_contains |

Réemploi uniquement si un futur GO revalide les mêmes bits, source/profil,
owner/nonceT3552 et dernière autoritéTERMINE ; dérive = HOLD, pas recompilation
ou import implicite. Aucune de ces invocations n’est autorisée maintenant.
La correction de raccord FS demandera une nouvelle source/ELF ; les anciens
ELF1ee ne peuvent pas exécuter une garantie qui n’est pas dans leur code.

**A — derived : entrée fixée non admissible, pas de run prédit.**
Linux2.6.0 copie privée `/tmp/lucivy-cmp`, archive4d0299..., manifeste complet
d4faf3..., origine/licence et noncecréateur e14 déjà archivés. Vue historique
attendue : DFS trié, c/h/rst/txt UTF8,1..60000octets, premier300, min50.
Liste300 et SHA par document déjà épinglés ; hash de la vue ordonnée attendue
2591b84244df8d7ee145f3f10df3aa6375c0cdc2d917a66481d5559f6552985b.
Ce hash n’est PAS une observation d’un nouveau payload consommé par le natif.
Le loader ignore encore des erreurs de dossiers/entries/metadata/read/decode et
ABSENT_OK peut générer du synthétique : rien de cela n’est devenu admission FS.

Le gate11 est après ce loader mais AVANT scratch et ses trois builders/reopen.
Le jeu300 fixé est connu sans mutex_lock (RAW/ancien rouge), donc nonadmissible
pour une preuve de parité. Un refus nommé serait rouge d’admission, jamais vert ;
C7 interdit explicitement de rejouer ce refus prédit. Aucun budget de nouveau
runA demandé, aucune permutation/extension pour rendre le panel compatible.

**B — ground-truth : qualification native HOLD justifiée.**
Copie privée publique Qcore/corpora/rag3db, commit1b22409b31a79543a9fe63e516c5b311bc43f1da,
8507 blobs réguliers, manifeste bb70899c..., licence LRSL1.2 et exclusions
.env.example/gitlink/symlink archivés. V3_CORPUS serait ce chemin précis,
jamais un working tree personnel. Pas de V3_MAX_DOCS ni V3_QUERIES changé pour
faire passer : cap déclaré5000, nombre et ordre réellement sélectionnés **inconnus**.
Le manifeste de tout l’arbre ne fixe pas à lui seul cette occurrence5000.

Le collector lit read_dir nontrié, entries.flatten, metadata=>0, read/decode
ignorés ; filtre NUL/taille100k/trim, et peut rendre une vue vide alors que la
racine existe. Le test retourne alors normalement. Son builder est créé avant
ses **quinze** queries, pas les onze derived : function(strict/relax), return,
struct, void, rag3db, include, uint64_t(strict/relax), std::unique_ptr(strict/relax),
ku_dynamic_cast(strict/relax), TableFunction(strict/relax). Leur liste exacte est
archivée sans la remplacer par11 ni par une sélection de hits.

run_panel peut compterOK quand référence et index sont vides ; ses oracles
relisent le FS avec skip/lossy/unwrap_or_default. V3_INDEX_DIR réutilise selon
une clé chemin/compte/options, ou remove_dir_all/persist à un chemin fourni.
V3_SPANS_REPORT_ONLY relâche les spans ; V3_DIAG peut lancer d’autres index.
Tous ces chemins sont hors du futur cas de correction. Ni un rc0 du vieil ELF
ni un contrôle externe de l’arbre avant/après ne certifieraient ses lectures
intermédiaires ou la vue réellement indexée. Pas de GO natifB proposé avec ces
lacunes, pas de 0=0 déclaré vert, pas de contact pour inventer le pin manquant.

## Contrat observable proposé, pas de nouvelle runtime

La frontière est un **snapshot FS admis pour un panel**, utilitaire de tests,
pas un moteur d’indexation supplémentaire. Entrée : origine admise, politique de
sélection versionnée, données de provenance et panel15 inchangé. Sortie : une
vue immuable ordonnée liant ordinal/chemin/empreinte aux octets exacts et aux
requêtes. Deux chemins ayant le même contenu restent deux occurrences ; un
chemin dupliqué dans le plan est refusé. UTF8/backmap partent des octets originaux.

1. Vérifier root/owner/provenance et exclusions avant lecture : jamais .env/.vault,
   lien ou chemin sortant de la racine. Ne pas appeler une lecture de plan
   « validation » si elle peut déjà ouvrir un chemin interdit.
2. Fixer la politique et cap AVANT résultat. Collecter la première vue admise une
   seule fois, enregistrer son ordre effectif et ses exclusions explicites.
   Ne pas prendre l’ordre du JSON des blobs pour celui de read_dir. Aucun second
   parcours pour chercher une autre vue après refus. Si une traversée triée est
   préférée, c’est un changement de politique à approuver AVANT corpus/résultat,
   pas une permutation de réparation.
3. Lire chaque document candidat au plus une fois, chaque retenu exactement une
   fois. Empreinte et décodage utilisent ce même tampon ; toute erreur IO ou UTF8
   d’un candidat texte est fatale, sans skip/continue/lossy/substitution. Une
   exclusion binaire explicite n’est pas une erreur UTF8 silencieusement avalée.
   Les mêmes octets doivent alimenter admission, builder ET oracle ; les oracles
   de correction ne rouvrent plus repo_path. Les mesures IO historiques restent
   des mesures distinctes, pas le fournisseur de vérité de ce cas.
4. Refuser la vue vide. Préparer les quinze requêtes et exiger un témoin nonvide
   pour chacune, avec les primitives pures déjà extraites ; publier toutes les
   absences, distinguer Read/Utf8/Query/Uncovered. Aucun builder/scratch/report
   d’index ni green0=0 avant cette décision.
5. Publier/fsync le manifeste effectif de vue et le panel avant builder, avec
   état ADMITTED ou REFUSED nommé. Un refus ne devient pas un test ignoré vert.
   Si le premier snapshot est refusé, il reste l’entrée fixée nonadmissible ;
   aucun nouveau snapshot après inspection des résultats.
6. Builder neuf en RAM sans capacité de reuse/persist/delete. V3_INDEX_DIR et
   les modes relâchant la preuve sont refusés avant effets, même valeur vide/0.
   À l’avenir la parité compare les mêmes ordinals et spans du snapshot ; la
   référence nonvide et les assertions de parité restent après admission aussi.

Le snapshot fixe une vue, pas une transaction universelle de la machine. Il doit
attester ses octets/ordre et refuser les dérives, pas prétendre que le FS entier
est immuable. Les anciens neuf verts gardent leur source/périmètre historiques.

## Vrais raccords proposés — signatures/chemins à relire, NON implémentés

Point consommateur natif exact :
`lucivy_core/tests/test_sfx_v3_ground_truth.rs::v3_ground_truth_contains`.
Le raccord proposé remplace son enchaînement collect_files→create_v3_index→
run_panel, pas un helper parallèle laissé inutilisé :

```text
v3_ground_truth_contains
  -> run_v3_contains_case(plan, StdFs, fresh_builder, compare_snapshot)
       -> collect/read-once -> freeze_view -> admit_panel(15)
       -> publish_view -> fresh_builder(view) -> compare_snapshot(view, panel)

quatre témoins de raccord
  -> LE MÊME run_v3_contains_case(plan, SpyStdFs, counter_builder, counter_compare)
```

StdFs est l’adaptateur concret vers les opérations réelles d’énumération,
métadonnées et lecture ; SpyStdFs délègue ces mêmes opérations sur de petits
fixtures temporaires et peut injecter une erreur déterministe au point read.
Pas de chmod000 supposé efficace sous root, pas de copie de collector réservée
aux mocks. Le spy compte ordre des opérations, nombre de reads par chemin et
appels builder/compare ; le vrai consommateur doit appeler cette entrée unique.

Baseline à extraire **sans corriger les quatre comportements** pour observer
les rouges. Déplacer les constantes du panel avant les closures ne signifie pas
encore l’admettre. Les erreurs avalées/retour vide/builder avant admission de la
baseline doivent rester visibles aux témoins. Une erreur de compilation ou une
API inexistante ne compterait pas comme rouge.

Puis correctif : composer un petit noyau admission-panel (PreparedQuery/oracles
existants), le wrapperderived gardant ses11 et B gardant ses15. Éviter de recopier
l’algorithme pour B ou d’élargir PreparedQuery en interpréteur de tout QueryConfig.
Si l’extraction modifie le chemin des neuf juges, un futur rejeu justifié sera
soumis au nouveau GO ; aucun rejeu n’est fait ici pour meubler.

Le builder de correction est extrait de la branche fraîche de create_v3_index,
sans try_reuse_index ni bloc persist/remove. Les autres benchmarks historiques
peuvent garder leur enveloppe explicitement horsqualification. Un spy de lecture
seul ne prouve pas zéro delete : la frontière d’effets builder doit être réelle,
source relue sans appel persistant, avec pièges/compteurs de reuse/delete dans le
raccord de baseline. La source corrigée ne reçoit pas de chemin d’index réutilisé.
Le report est un sink neuf autorisé après admission, sans tronquer une preuve
existante du target ; le chemin compilé common::report_path n’est pas TMPDIR.

## Quatre vacuités red-first préparées sur le consommateur

Les données sont des petits fixtures FS temporaires conçus pour le témoin,
jamais les corpus publics actuels et jamais un changement posthoc de ces corpus.
Les quinze queries de B sont partagées avec le vrai cas, pas une sous-liste facile.

| Vacuité | Entrée/spy au même raccord FS | Rouge baseline attendu | Garantie après correction |
| --- | --- | --- | --- |
| Vue vide | Racine fixture existante, aucune entrée retenue | Succès/retour vide au lieu du refus nommé | EMPTY_VIEW, zéro builder/report/compare |
| IO échouée | Entrées réelles, spy injecte erreur enumerate/metadata/read sur un candidat ; d’autres fichiers valides existent | Skip puis vue incomplète/succès au lieu de l’erreur de source | IO_ERROR avec chemin/opération, aucun remplacement par le fichier suivant, zéro builder |
| UTF8 invalide | Fichier texte sélectionné avec octets invalides, empreinte connue | Skip/lossy au lieu du refus de décodage | UTF8_ERROR sur ce fichier, jamais contenu vide, zéro builder |
| Panel noncouvert/0=0 | Vue nonvide valide sans témoins des15, comparaison mock vide des deux côtés | Builder appelé et égalité0=0 acceptée | UNCOVERED_PANEL précis avant builder, aucune assertion retirée |

Compagnons dans ces témoins : un jeu FS positif séparé couvre15 et passe une
seule fois au builder ; chaque document retenu a read_count=1 pour hash/décodage/
index/oracle ; le manifeste ordonné est publié avant ce passage. Muter le backend
après snapshot ne doit déclencher aucun reread. Injecter V3_INDEX_DIR ou un mode
relâchant doit refuser avant lecture/effet ; les compteurs reuse/delete/persist
restent0. Ne pas fabriquer quatre verts en testant seulement un helper détaché.
Les quatre rouges devront être réellement vus sur la baseline publiée, avec
leur cause précise, avant le correctif ; aucun de ces témoins n’existe ou n’a
été exécuté dans cette préparation documentaire.

## Coût et séquence à proposer après RO

Coût source estimatif, pas un diff livré : raccord unique+adaptateur FS minimal,
extraction du builder frais et référence-snapshot, quatre témoins/spies. Pas de
nouvelle crate runtime ni service. Une empreinte SHA256 doit être calculée sur
le tampon lu, pas via DefaultHasher. Les manifests directs core ne déclarent
pas SHA256 ; le graphe compilé archivé contient déjà sha2@0.11.0/digest@0.11.3.
Une dépendance **de tests** explicite pourrait réutiliser ce graphe, à vérifier
sous Cargo offline lors d’un GO futur ; ce n’est pas une attestation du cache
actuel. Pas de code crypto artisanal ou d’hypothèse de nouvelle résolution.

1. RO de ce contrat/raccord/spies et décision sur politique de capture/transport
   du manifeste. Le vieux V3_CORPUS seul ne transporte pas une vue ordonnée : le
   chemin d’un plan de test explicite, ou la capture initiale publiée avant
   builder, doit être défini. Aucun nouveau flag runtime n’est introduit ici.
2. Après autorisation distincte : extraction baseline, sources/tests/recette
   publiés avant exécution. Go rouge CPU/FS fixtures borné à proposer : budget
   de compilation séparé (Tunique32G/quota4), puis quatre cas discriminants
   déclarés, ou une table agrégée dont chaque vacuité est observable. Aucun
   vrai corpus/natif. Tout autre écart STOP sans retry ; pas un rouge d’API absente.
3. Correction seulement après vrais rouges ; nouveau source+RO+GO vert pour ces
   cas et contrôles positifs. Plafond proposé300s dur (validation285) si compilation
   nécessaire ; le temps de tests petits fixtures seul doit rester sous60s.
   Aucun GO de ces étapes n’est demandé implicitement par le plan.
4. Reprise native seulement après admission observable. A reste fixed-input
   nonadmissible tant qu’aucune nouvelle entrée légitime n’est autorisée ; pas
   de nouvel essai de refus prédit. B reçoit une source/ELF nouveaux, un snapshot
   figé et panel15 nonvide, sans réutilisation V3. Recette native/ressources/ledger/
   profils/manifestes/finalwhole seront relus avant nouveau GO. Les anciens pins
   ELF1ee restent archives, aucun budget natif actuel n’est proposé ou lancé.

Aucun contact corpus, aucune source Rust changée, aucun témoin exécuté ici.
Source1ee/neuf mémoire verts, core/9HOLD/FS/ULP7.2/PR42 gardent leurs périmètres.

## RO contrat reçue et choix restant avant code

Recherche exacte355ffadd01 favorable bornée, après lecture doc16/JSON/ticket et
collect_files/create_v3_index/v3_ground_truth_contains/run_panel1ee. Aucun
Rustdiff/test/Cargo/ELF/proc/corpus/service. Pas de bloqueur statique du contrat ;
StdFs/SpyStdFs au vrai consommateur avant effets, quatre baselines conservées,
positif séparé et même tampon hash/UTF8/index/oracle confirmés. Les pièges
reuse/delete/persist doivent viser la vraie frontière d’effets, et les flags
V3 dangereux être refusés avant lecture même valeur vide/0.

Choix à faire par C7 avant implémentation/GO : capture de la première vue et
transport de son plan. Proposition minimale : origine/arbre/politique/cap/panel
figés avant lecture, capture unique publiée avant builder, refus conservé sans
seconde sélection ; le manifeste effectif sert de référence de replay ensuite.
Ne pas introduire un tri qui changerait silencieusement la sélection5000, ni
une nouvelle runtime autour du refusA interdit. Un plan de cas explicite peut
porter ce contrat dans les utilitaires de tests ; son point de transport doit
être validé, le vieux V3_CORPUS seul ne le porte pas. Aucune de ces options
n’est implémentée ni autorisée à lire les corpus dans cette préparation.

RO secondaire Banc ccd9ebcf86, `docs/banc/16-11-2026/23-ro-contrat-admission-fs-lucivy.md`,
favorable au contrat355ff :4SHApins conformes, baseline1ee relue viaGit,
aucun bloqueur supplémentaire. Si l’on revendique les trois erreurs IO,
enumerate/metadata/read exigent des sous-cas distincts observables : un seul
read injecté ne vaut pas preuve de tous. ManifestREFUSED autorisé comme preuve
d’admission, jamais comme report/index créé avant gate. Source/capture/transport
à figer par C7 avant code ; aucun impl/témoinCPU/Cargo/ELF/proc/contact corpus
ou service par Banc. Les budgets300/60 restent propositions, pas GO.
