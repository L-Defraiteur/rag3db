# Arbitrage Cargo avant matérialisation du pilote complet

APP figée `274803c9241b59523ac4026180d9c20f3aa03091`, base
`bef0b27cb33b9143dabee019fccdf7453da553a3`. Cette page précise et corrige
la recette16 ; préparation seulement, aucun GO7200. Les couples du13novembre
et leurs échéances sont des archives, jamais des autorités réutilisables.

C7 autorise un dossier `task-targets` initialement absent SOUS le target de
session existant. `RAG3WEAVER_TARGETS_DIR` y pointe ; le véritable calcul
`cible_du_depot` fournit les sous-cibles des tâches. Chaque propriétaire est
déclaré et la garde commune réelle précède Cargo. Ni seconde racine de cache,
ni clonage d'artefacts, ni purge, ni faux marqueur. L'ancien inventaire est
comparé intégralement en excluant le seul enfant natif déjà actif et le nouvel
enfant explicitement absent avant préparation. Tout état préexistant dans ce
nouvel enfant est un refus. Le cumul cible de session + fichiers privés F reste
borné à32Gio ; le cgroup commun32Gio et l'échéance unique restent applicables.

Le shim documentaire reste privé sous Q ; sa lecture est ajoutée explicitement
à `workspace.sandbox.extra_read` dans la SEULE configuration de la fixture
D6. Le Cargo réel continue dans Landlock ; un contrôleur extérieur peut
admettre/observer, jamais exécuter la commande en dehors du bac. Aucun fichier
posé sous `~/.cargo/registry`, aucun binaire/configuration partagés remplacés,
aucun accord générique. Le shim n'est pas ajouté au PATH de la lib canonique.

Le témoin `commande::tests::cargo_plays_its_tests_inside_the_sandbox` reste
inchangé : vrai Cargo, vrai crate jouet, vrais tests et refus d'écriture hors
bac. Ses TempDirs sous TMP privé sont comptés avec F, sans cache conservé par
essai. Son crate sans dépendance native ne fait pas disparaître les variables
SDK : `commande::lancer` hérite l'environnement, sans `env_clear`, et ajoute
seulement `sandbox.env`. Profil de compilation et environnement sont donc
déclarés séparément avant lecture ; aucune politique choisie selon les valeurs
observées. Profil exact de la lib hôte, modifications Cargo/Rustup du chemin LD
et cwd/target du jouet doivent être reliés à la même ascendance possédée.

L'appel Cargo d'une tâche suit la compilation APP et ne chevauche aucun rustc
de l'APP. Un parent Cargo qui attend un ELF de test n'est pas en soi une
compilation APP active. Toute admission d'un sous-target lie commande, source,
cwd, propriétaire, identité/portée, features et profil attendu avant lancement.
Commande inconnue : diagnostic et arrêt, pas un échec synthétique substitué au
vrai Cargo demandé par le scénario.

Le module de politiques préparé ensuite n'est pas encore un pilote complet ni
une entrée autorisée. Restent à matérialiser/revoir : admission poste/autorité
fraîche, observation de tous Cargo, ledger avant runtime, D6 et ses juges,
canonique entière, finalisation/copies/deadline après ouverture. Ses témoins
hermétiques seront publiés avant exécution ; aucun Cargo, modèle ou service
n'est nécessaire à cette préparation.
