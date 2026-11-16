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
