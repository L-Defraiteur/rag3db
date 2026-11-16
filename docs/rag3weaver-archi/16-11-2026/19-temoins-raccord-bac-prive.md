# Préparer le rouge du raccord Bac privé des fixtures

C7 autorise une préparation des seules fixtures de test depuis APP274.
Le ticket existant est complété AVANT ce diff. Aucun GO Rust ou7200.

Deux témoins à écrire appellent les vrais setups `depot_de_test` (e2e) et
`depot_inscrit` (lib). Le parent crée une déclaration sandbox privée depuis
le gabarit livré, y ajoute un dossierQ privé en lecture, puis relance son
propre ELF pour ce seul témoin avec la déclaration en environnement.
Le sous-processus appelle le vrai setup et compare le sandbox du manifeste
de sortie ET de son commit initial à la déclaration. Cela vérifie le point
avant création de tâche et montage du Bac, sans simuler la fonction setup.
Le processus enfant évite de modifier l'environnement des autres tests.

Le contrat de préparation sera `RAG3WEAVER_DOCUMENTATION_SANDBOX_FILE`, lu
uniquement par les helpers de test après le rouge et le correctif autorisé.
Le contenu est la configuration `workspace.sandbox` existante, pas une
nouvelle capacité du runtime. Les setups actuels n'utilisent pas cette
déclaration ; les assertions attendent un refus nommé pour ce raccord absent.
Le parent archive stdout/stderr de l'enfant dans sa fixture conservée à
l'échec. Ni mock du résultat Cargo ni commande Cargo exécutée par le témoin.

Cette publication préalable contient seulement le ticket et la recette.
Témoins à publier séparément avant compilation/exécution ; correctif absent.
Lourd court à préparer sur le même cache de session, ledger/copies avant
juges, SDK59 exact, garde et échéance neuve, arrêt au premier rouge. La
campagne complète attend toujours nouvelle APP/pilote/RO/GO distincts.

## Témoins matérialisés, NONJOUÉS

La publication suivante ajoute seulement les deux entrées de test et le
helper d'observation dans trois fichiers déjà touchés par1bd. Les vrais
setups restent STRICTEMENT inchangés. Le helper lance le même ELF et le même
nom exact dans un enfant isolé ; l'enfant effectue le setup, sans hôte,
montage, service, modèle ou commande Cargo. Le parent garde ses deux sorties
en fixture à l'échec. Les deux noms sont :

- lib : `daemon_host::teammate_tests::task_fixture_uses_the_declared_private_sandbox_before_add` ;
- e2e : `documentation_fixture_uses_the_declared_private_sandbox_before_add`.

Une invocation externe de chaque nom fait une seconde invocation interne du
même nom, typée sous-processus de préparation, pas un second juge distinct.
ROUGE attendu : `DOCUMENTATION_PRIVATE_SANDBOX_NOT_CONNECTED_BEFORE_ADD_FILES`
dans l'enfant, repris par `DOCUMENTATION_PRIVATE_SANDBOX_SETUP_REFUSED` dans
le parent. Compilation cassée, zéro test, autre panic ou timeout ne sont pas
ce rouge. Pour observer les DEUX absences avant tout correctif, une future
recette doit demander explicitement deux rouges attendus sous une même borne,
avec arrêt au premier défaut inattendu ; aucun droit de poursuivre un rouge
inattendu. À défaut de cet arbitrage, un seul nom pourra être lancé.

Contrôles effectués sans exécution des témoins : rustfmt sortie standard
jetée pour chacun des trois fichiers (parse seulement), diffcheck,656tickets.
Aucun Cargo/rustc, aucun ELF de ce source ni résultat de test revendiqué.
