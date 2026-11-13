# La garde attribue un worktree au cwd après deinit

- **État** : ouvert ; retrait luciepc suspendu sans force, sources et administrations archivées.
- **Famille** : produit
- **Gravité** : bloque le retrait sûr d'un lot livré et conserve son target24,4Go.
- **Atteignable en service** : non, procédure de nettoyage des worktrees.
- **Touche rag3weaver** : oui, scripts/garde_worktree.py.
- **Pour** : recherche seule pour le correctif garde CPU ; arbre-principal triage lecture seule et raccord physique ; embarquement seul pour le retrait de son lot après qualification.
- **Signalé par** : embarquement, retrait après livraison23ceed87b/7308544ce.
- **Ouvert le** : 13 novembre 2026

## Constat et preuve

Le sous-module codeparsers propre avait un git-common-dir privé sous
.git/worktrees/cx-activation-modeles/modules/extension/rag3weaver/codeparsers,
un seul worktree, aucun ignoré/local, zéro référence active W/target.
Administration entière archivée fidèlement AVANT et APRÈS git submodule deinit
--all sans force. Deinit supprime core.worktree ; git --git-dir ADMIN worktree
list rapporte alors le cwd du processus, ici le dossier d'archives externe.
Le garde le compte comme autre worktree et rend ROUGE. Aucune suppression du
worktree/target ni aucun contournement du garde n'a suivi.

Preuve PC : ~/.cache/embarquement-notes/activation/archives-fin-luciepc/garde.json.
Refus exact : « administration sert un autre worktree :
/home/lucied/.cache/embarquement-notes/activation/archives-fin-luciepc ».
Les archives avant-deinit, finale, données générées et references.json sont
conservées ; le portable a été retiré avec garde VERT et reçu indépendant.
WPC /home/lucied/git_workspaces/cx-activation-modeles, target-spans24407724032.
Ce dossier d'archives n'est pas un worktree véritable du sous-module : son
attribution change selon le cwd implicite de Git après désinitialisation.

## Suite et critères de fin

Arbre/coordination : déterminer le traitement explicite d'une administration
résiduelle désinitialisée, sans inférer de source depuis le cwd, masquer un
partage ni changer core.worktree pour faire passer la garde. Préserver les
archives et le rouge ; preuve discriminante initialisée/désinitialisée/partagée,
rapport frais hors cible et VERT avant retrait. Embarquement reprend alors son
retrait quiescent sans force. Ne pas effacer/pruner une administration à la main.

## Prise en charge recherche avant édition du garde

C7 attribue explicitement le correctif garde CPU à recherche, worktree/branche neuf `cx-recherche-garde-deinit`, base master798ab1aa7. Arbre reste en triage RO et garde son raccord physique, Embar garde seul le retrait de son lot. Aucun W/admin/target Embar ni réglage core.worktree modifié par recherche. Embar a confirmé avant-deinit/admin-finale/rouge/references conservés, un seul checkout privé propre avant deinit et aucune mutation depuis refus.

Lecture du garde : `worktrees(common)` appelle d’abord `git --git-dir common worktree list --porcelain -z`, puis ne traite le cas sous-module que si le chemin rendu est égal au common-dir. Après deinit supprime core.worktree, Git peut déjà rendre le cwd extérieur en première réponse : la condition de fallback n’est plus atteinte. Le défaut n’est donc pas seulement show-toplevel ; le premier chemin n’est pas une identité de checkout probante. Ce diagnostic statique est cohérent avec le rouge Embar, à matérialiser dans un contre-exemple Git isolé avant changement produit.

TDD préparée : fixture parent+worktree+module local committé, deinit --all sans force, archive administrative fidèle ; mêmes demandes du garde depuis deux cwd extérieurs privés. Attendu contrat : VERT dans les deux cwd pour administration prouvée désinitialisée/non partagée et archive fidèle, aucune attribution du cwd ni mutation. Attendu rouge de l’ancien garde798 : cwd inventé comme autre worktree. Source/recette publiées AVANT le test ciblé borné60s, preuve structurée du Git brut et des deux rapports hors fixture. Le rouge attendu est le contre-exemple TDD, aucun essai sur ressources Embar.

Modèle à implémenter seulement après constat : distinguer checkout initialisé attesté, administration résiduelle de sous-module attestée par gitlink/.gitmodules et stockage privé attendu, et inscriptions linked worktree. Une absence de core.worktree seule ne prouve pas un deinit. Cwd/config override interdits, aucun contrôle de partage/archive supprimé ; état non attribuable, inscription liée, contenu résiduel ou configuration ambiguë restent conservativement refusés. Les cas initialisé/linked/partagé et fidélité des archives restent requis. Aucun Cargo/GPU/service/ancien runtime, publication pour revue C7 avant master ; Embar attendra nouveau garde intégré et rapport frais VERT pour son propre retrait.

## Contre-exemple observé puis proposition d’ancrage

Source1c30ec11b publiée avant exécution ; témoin ciblé60s : **rouge attendu**,1test échoue en0,181s, deux gardes observés ROUGE attribuent respectivement cwd-exterieur-a et-b ; sources/administrations inchangées et archives reconnues fidèles. Preuves locales hors dépôt `journaux-cx-recherche-garde-deinit-13nov/{rouge-contre-exemple.log,rouge-code.txt,rouge/deinit-cwd.json}`. Git local2.56.0. Le diagnostic brut initial sans -z montre l’administration, la garde finit au cwd via le fallback ; la fixture courante enregistrera aussi la variante exacte -z/gc.auto/GIT_OPTIONAL_LOCKS de la garde. Le fait Embar « premier appel peut déjà donner cwd » reste distinct : on ne déduit pas son interleaving d’un stdout brut non conservé.

Correctif proposé avant nouveau test : ne plus demander à Git --git-dir de deviner un checkout (ni worktree list ni show-toplevel). Reconstruire le principal depuis core.worktree explicite ou .git réel ou source/gitfile connue, en vérifiant les allers-retours source→admin ; reconstruire les linked depuis common/worktrees et gitdir/commondir réciproques, refuser manquants/verrouillés/dupliqués. Si aucun principal, seule une preuve .gitmodules+gitlink160000+nom/rangement privé attendu+source vide/absente permet l’état désinitialisé ; toute inscription liée reste refusée, absence de core.worktree seule insuffisante. Rapport explicite etat/ancrage ou preuve_gitlink ; inventaires/archives fidèles et contrôles de partage restent complets.

Configurations incluses, multiples/ambiguës, bare ou source non attribuable refusées, aucune config changée. Les contrôles .env/.vault/symlinks/dirty/archives ne sont pas abaissés. Témoins ajoutés : deinit privé invariant deux cwd, deinit partagé lié conservé, deinit non vide, gitlink absent, archive périmée, configuré sans gitfile, linked verrouillé/endommagé, administration inconnue sans core, interdiction interne de checkout inféré par --git-dir. Recette prévue : même ciblé puis suite CPU Git complète bornée60s, source publiée d’abord ; premier autre rouge STOP/diag sans retry inchangé. Aucun essai sur W/admin/target Embar, qui attend revue/intégration et garde fraîche.
