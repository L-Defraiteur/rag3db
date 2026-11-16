# Raccord commun des deux fixtures avant add_files, vert proposé seulement

Préparation TESTS autorisée C7 APRÈS [les deux rouges réels22](22-preuves-deux-rouges-bac.md)
et leurs2111fichiers répliqués/9d82SHA publiés480604 AVANTce diff. Lot et
branche source existants, aucune nouvelle API de production, aucun Cargo,
rustc ou témoin natif exécuté pour ce correctif. Source223/049/1bd, DDE6 et
72338/fixtures/ELFs/rouges restent figés surPC et dans leurs archives.

## Frontière partagée

`tests/common/documentation_fixture.rs::connect_declared_sandbox<S>` est
TEST-ONLY. Les deux vrais setups l'appellent avec le VRAI
`backend_code::SandboxConfig`, dont la désérialisation existante refuse les
champs inconnus. La déclaration privée explicite vient de
RAG3WEAVER_DOCUMENTATION_SANDBOX_FILE ; absente, le setup normal est inchangé.
Pas de nouvelle structure Sandbox ni extension du runtime.

Le helper exige fichier canonique/régulier et RAG3WEAVER_TARGETS_DIR
explicitement posé sur un répertoire canonique existant. Il conserve
Landlock, réseaufermé, aucune extra_write, commandeapproval, tokens
{repo_target}/{task_dir}/tmp et Cargooffline. Jobs déclarés positifs et
≤MIN8/POSTE_FILS ; le témoin déclare1 pour rester valide sur toute place.
La seule donnée remplacée est workspace.sandbox, validée puis recopiée
VERBATIM (pas ajout involontaire de champs pardéfaut). Ni set_var global,
ni shimregistry, ni modification de Cargo/main/locks/gitlinks/templates.

`chat_qui_code::depot_inscrit` et `e2e_equipier::depot_de_test` utilisent la
MÊME primitive avant écriture du manifeste source puis `code_tree::add_files`.
Celui-ci copie le manifeste et crée son commit de déclarations ; aucune
correction après création de tâche. Le module partagé devient pub(super)
seulement dans les tests existants ; pas de duplication des cinq tests de
policy. chat_qui_code et teammate_tests sont déjà cfg(test,unix,code), le
raccord de leur Sandbox est également cfg(rag3db-native).

Le témoin parent crée une cible privée VIDE et la passe UNIQUEMENT à son
enfant par Command.env, sans modifier le processus des autres tests. Cette
racine ne reçoit aucun Cargo : le témoin ne fait qu'appeler les vrais
setups/commits avant tâche. Les targets de session du futur parcours Cargo
restent ceux du mécanisme repo_target sous Tsession, admis par son porteur ;
ce petit répertoire de fixture ne constitue pas un second cache de build.

## Témoins et traces vertes attendues

Les deux noms réellement vus rouges sont conservés :

- `daemon_host::teammate_tests::task_fixture_uses_the_declared_private_sandbox_before_add` ;
- `e2e_equipier::documentation_fixture_uses_the_declared_private_sandbox_before_add`.

L'enfant lit la déclaration, appelle le VRAIsetup, compare le sandbox du
manifeste ET de son HEAD à cette déclaration et vérifie commands=approval.
Après ces assertions, il publie DOCUMENTATION_PRIVATE_SANDBOX_COMMITTED
avec nom/manifeste/HEAD/extra_read/tokens/jobs/offline/targetsroot et deux
égalités. Champs positifs seulement, jamais l'environnement complet.
Le parent exige le vrai résumé enfant1/0/0, puis rend visibles ses sorties
sous DOCUMENTATION_SETUP_CHILD_OK ; un résultat vert ne cache donc plus la
seconde invocation. Échec : conservation complète identique aux rouges.

Protocole vert futur pour CHAQUE nom : rc0, parent1réussi/0échec/0ignoré et
enfant1réussi/0échec/0ignoré, deux noms identiques qualifiés, exactement deux
sommaires, marker CHILD_OK et unique JSON COMMITTED concordant. Aucune
panic/FAILED/ignoré/nomabsent ou zéro test toléré. Deux juges distincts,
quatre invocations brutes ; inventaires quatre exécutions NONjuges. Les
TempDirs réussis sont normalement libérés ; traces JSON/stdout enfant sont
visibles avant leur retrait, pas de fixture verte reconstruite après coup.

## Recette verte à matérialiser après RO source, aucun GO implicite

Une future autorité NEUVE est proposée, pas créée : commun1800MAX après
vrai poste et globalTERM1920/KILL1935 comprenant l'attente et la finale.
MINENDcommon/soft par phase, final≤120/solde/postopen. Les END1197212/1197227
du rouge sont expirés et ne s'utilisent pas pour le vert. Nouveau couple
source/pilote et GO exact distinct requis, même branche/lot et UNIQUE T.

Avant mutation : propriétaire courant source223 et scope72338 attenduROUGE101
TERMINE/nettoyé/restes[]/bus+cggone,2111SHA et2ELF223 originaux/copies/ledger,
130owned disparus, source/pilotepropres/gitlinkbd, cachesource/préparation
stables. Préparer copiephysique223/admin/gitlink avant transitionGitseule
au même C ; inventaire T et ancienétat racine identiques avant/après Git.
Aucune purge, cible neuve ou ancienELFréattribué. La garde commune réelle
et son préflight restent dans la future autorité, pas dans cette préparation.

Ordre proposé : préflight/garde30+30 → buildlib600 puis buildtargete2e600
avec no-default/code,daemon,rag3db-native/offline/locked/no-run/JSON →2artifacts
uniques/2prefixessource/SHA/copies+4inventaires15(nonignorés) →ledger+2ELF
répliqués et réhachés DEUXpostes/pause≤120AVANTpublication+marker →2exacts45
avec protocole1/0/0 ci-dessus →scopeVERT0/ownedquiet/source/archives/SDK/
ELF/ENDaprès hashes et aprèsopen. Caps intersectés, aucune durée promise.
Premier rouge INATTENDU STOP et preuve conservée, aucun retry automatique.

SDK59/store réel/deuxSO+header pins, exportUNEfois par phase, mémoire32G/
HIGHmax/quota≤8/jobsMINposte/Rusttests1 et racines privées courtes surdisque/
étatsXDG-RAGfrères restent obligatoires. Pas de nouveau modèle/service,
API7970, DBpersonnelle, observateurCargo/procstop ou GDB ajouté pour masquer
une preuve historique absente. Pas de maps/main, ABIglobale, absenceGPU,
D6/Vertex réel/F8/libentière/NAV qualifiés par ces deux setups.

À cette publication : rustfmt PARSE des quatre fichiers/diffcheck seulement.
Les tests corrigés et le protocole vert sont ÉCRITS/NONJOUÉS, la production
et les montages physiques ne sont pas déclarés qualifiés. Revue source puis
pilote/RO/GO distinct avant tout natif ; ticket reste OUVERT.
