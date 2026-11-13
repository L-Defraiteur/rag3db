# La garde attribue un worktree au cwd après deinit

- **État** : corrigé par e167681867390914610cf3fe6e9560c6f3ca1f30 ; garde réelle fraîche VERT et retrait W/target achevé sur les deux postes.
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

## Première qualification locale de la source ed7607325

Source et recette publiées avant tests. Contre-exemple désormais **VERT**,1test0,198s, deux cwd extérieurs sans attribution ni mutation. Puis suite entière **29/29 verts en2,255s**, bornée60s/failfast, sans Cargo/service/GPU ni ancien runtime. Les tests antérieurs de protections, liens, partage et archives restent verts, de même que les nouveaux refus conservatifs. Les états désinitialisés produisent une preuve gitlink explicite et zéro checkout, pas un chemin cwd ignoré arbitrairement.

Prochaine qualification autorisée dans ce même périmètre CPU : même source ed7607325, worktreecx neuf sur luciepc, fixtures privées sous son dossier de preuves ; `POSTE_MEM_MAX=1G poste lourd timeout60 python3 scripts/test_garde_worktree.py`, log/observations hors worktree. Cela compare le comportement Git2.56.0 du poste affecté sans jamais lancer le garde sur W/admin/target Embar. Source/recette publiées avant cette qualification, premier rouge inattendu stop/diagnostic. Embar seul fera une garde réelle fraîche après revue et intégration du correctif.

## Qualification CPU luciepc et preuves consolidées, avant master

Source exacte **ed7607325f3796534b07625a16340ff40f212afe**, garde SHA256 **6f3dfceb17c9703029d2f4f26385f9aa21820c84bc79c518e1338b6d0f36188c**. Git2.56.0 sur les deux postes. Portable : ciblé1/1 en0,198s puis suite29/29 en2,255s ; luciepc : même suite29/29 en2,454s, sous poste lourd effectif1G/timeout60, fixturesGit privées et failfast. Aucun autre rouge que le TDD attendu1c30, aucun retry inchangé, aucun Cargo/GPU/CDP/service/ancien lot.

C7 et mémoire ont relu ed760 statiquement : principal ancré, linked réciproques, résidu positif sans source/CWD, partage/ambi/archives préservés ; aucun défaut concret signalé. La preuve locale du ciblé/CPU et la preuve PC portent cette même source (commits suivants uniquement Markdown). Le garde n’a PAS été essayé sur le worktree/admin/target Embar ; ceux-ci restent intacts. Le vert réel frais appartient au porteur après intégration, pas à cette qualification de fixtures.

Preuves à conserver aux deux postes sous `~/.cache/rag3weaver-build/journaux-cx-recherche-garde-deinit-13nov/` : rouge-contre-exemple.log/rouge-code.txt et rouge/deinit-cwd.json, vert-cible.log/vert-cible/deinit-cwd.json, suite-cpu.log/suite/deinit-cwd.json, pc/{source.txt,git-version.txt,garde.sha256,suite-cpu.log,deinit-cwd.json}. Les rapports de fixture montrent l’état désinitialisé sans checkout et la preuve gitlink ; aucune archive ou source mutée par la garde. La procédure décrit le nouvel ancrage et la reprise propriétaire, sans overrideCWD/config ni contournement du refus.

Soumission C7 : revue du diff/procédure/ticket et tour master requis ; ticket maintenu ouvert pour intégration puis garde fraîche Embar avant son retrait. Recherche n’efface aucune ressource d’Embar et ne remplace pas son rapport gardé. Arbre continue son raccord physique sans édition garde.

## Résolution et retrait effectif

Helper intégré e167681867390914610cf3fe6e9560c6f3ca1f30, SHA256
6f3dfceb17c9703029d2f4f26385f9aa21820c84bc79c518e1338b6d0f36188c,
qualifié29/29 CPU sur les deux postes puis extrait hors cible et re-vérifié.
L'état désinitialisé est prouvé par les ancrages Git, sans source déduite du cwd.
Aucune configuration/index du worktree Embar n'a été modifié pour passer.

Preuve réelle PC : ~/.cache/embarquement-notes/activation/
archives-fin-luciepc-e167-final/{garde.json,administration-fidele,
quiescence-avant.json,quiescence-apres.json,retrait.json}.
Archive neuve fidèle, rapport neuf VERT/refus[], puis quiescence immédiate.
Le refus initial de Git worktree remove était structurel (sous-module), après
une garde verte, distinct d'un refus de garde. L'orchestration a confirmé le
bornage de la procédure : UN remove --force pour ce seul W, aucun double force,
aucun rm manuel d'administration ni branch -D. Autorisation consignée AVANT
l'action dans activation/bornage-retrait-e167.txt. Branche supprimée par -d.

Le scan ne trouve aucune référence lisible vers W/target ; les unités propres
ont MainPID0. Les FD protégés du bureau/système/transport restent nommés
inconnus externes dans les preuves, jamais publiés zéro. Deux précontrôles
stricts arrêtés avant garde/archive sont conservés ; aucune suppression avant
le dernier verdict frais. Ancien rouge et toutes les archives restent intacts.

Absences W et target vérifiées sur portable et luciepc après retrait ; le reçu
portable initial est activation/archives-fin-portable/retrait.json. Sources138,
binaires mesurés, preuves GPU, rouges et branches distantes sont conservés.
Fixtures PG de l'union : bases0 et sessions0, contrôlées après retrait.
Aucun Cargo, modèle, service ou GPU relancé pour ce nettoyage.
