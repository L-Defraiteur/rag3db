# Pilote exact du diagnostic des deux rouges

**PRÉPARÉ, NON ADMIS, AUCUN C++ COMPILÉ OU JOUÉ.** Sources de test revues favorablement par Hotfixs avant l'ajout identique des impressions BUILD_ID/configuration. Le présent pilote et ces impressions attendent leur revue RO et un GO exact distinct de C7. Aucun staging sur PC, poste ou unité créé pour cette préparation.

## Couple de sources, distinct du pilote

- Candidat : `ed0c3c8af57f25e83ae5c4ba004ae3393e77812e`, branche cx-coeur-two-reds-diagnostic, moteur888c81825e4cd6556261233f0e238af3b95375c9.
- Référence : `75a4ba4d467aa5f07ef83494f5393154ab2392c6`, branche cx-coeur-two-reds-baseline, moteur59ac9c86bc2b3529b590a7462ab7051e3ad26b8a.

Les deux fichiers test restent identiques entre variantes. L'ajout depuis la paire a3f3/50412 émet le macro BUILD_ID réellement compilé et pool/fils configurés ; aucune modification des assertions ou du mécanisme de barrière. Le delta des chemins moteur reste dictionary_chunk.cpp seul. Ne pas confondre source/pilote/moteur. Le profil attendu est pour chacun Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF, LTOOFF. Aucun SDK installé n'est chargé : chaque variante construit son archive statique et son ELF concurrence privé ; la lib59 de l'hôte n'est pas une preuve de ce build référence.

[Recette exacte, SHA des scripts/bundle/tests/dictionnaires](outils/two-reds/recipe.json). Bundle privé local ~/.cache/rag3db-moteur-notes/two-reds-preparation/source.bundle, deux refs, ancêtre requis64eee2e1f6c3599f5e4a0bcd3369f6202e33eaca ; copie distante seulement après admission. Aucun besoin de changement SDK/lib/service.

## Entrée proposée, après revue et GO uniquement

J neuf : ~/.cache/rag3weaver-build/cx-two-reds-01. Scratch extérieur neuf et vide : ~/.cache/rag3weaver-build/cx-coeur-two-reds. Unité attendue absente : cx-coeur-two-reds-01.service. Copier les fichiers du dossier outils/two-reds et le bundle, vérifier SHA/mode/absence de reçus précédents ; refus sans réessai à la moindre divergence.

Commande exacte : `bash /home/lucied/.cache/rag3weaver-build/cx-two-reds-01/launch.sh`. Elle entre par le vrai poste lourd32G/swap0, sans forcer quota ou fils. Le watchdog extérieur commence après acquisition : TERM2390s, KILL dix secondes après, soit2400s au maximum. entry.py lit cpu.max et POSTE_FILS, j=min(8,fils,floor(quota)), quota enfant égal au parent effectif.

Délais absolus depuis acquisition : contrôleur2250s ; unité RuntimeMax<=2260s avec arrêt10s, fin native<=2270s ; finalisation souple2380s, avant le watchdog2390/2400s. Toute phase prend le minimum de son plafond et du temps contrôleur restant moins10s. Configuration120s/build800s/cas60s par variante. Les320s non affectées aux plafonds nominaux de configuration/build/cas couvrent aussi staging, exports et finalisation ; elles ne s'ajoutent pas au budget. Un temps de staging élevé raccourcit les phases suivantes.

Une portée réseau vide, tmpfs privé32G, mémoire32G/swap0, sources RO, build RW uniquement pendant configuration/compilation, build RO pendant les tests ; tmp/home privés, environnement tests nettoyé. Extensions non construites : les deux cas ciblés n'en chargent aucune. Un seul target concurrence_test. Aucun known_red complet, transaction, E2E, Cargo, GPU, PG, modèle ou service.

## Premier rouge : arrêt, même s'il était attendu

Ordre exact : candidat chaînes instrumentées, candidat lecteur contrôlé, référence chaînes instrumentées, référence lecteur contrôlé. Une exécution par cas, aucun retry. Le ledger initial déclare les quatre NON JOUE et est conservé à chaque transition.

Un GTest rouge complet (code1 cohérent avec son XML nominatif) est conservé puis arrête immédiatement toute la paire. **Il ne permet pas de poursuivre pour obtenir le contraste référence/candidat.** Les cas suivants restent NON JOUE. Si le candidat chaînes redevient rouge, aucune référence ne sera construite par ce lancement : toute mesure comparative manquante demandera une recette distincte explicitement admise, pas un réessai automatique. Ce choix applique la dernière consigne C7 de premier rouge STOP et remplace la proposition de poursuite diagnostique du document12.

Timeout/crash, code inattendu, absence d'identité, cas absent/sauté/désactivé, XML incohérent ou descendant résiduel arrêtent aussi la paire. Aucune exception de rouge historique, aucun changement de known_red ou de statut des anciens témoins. Un éventuel VERT final ne qualifie que les quatre sondes nommées, jamais la concurrence globale ni C1.

## Identités, archives et fermeture

Avant compilation : SHA source, propreté, dernier commit des vrais chemins build_id, deux fichiers test et dictionnaire vérifiés, commandes/sorties conservées. Après compilation : ELF, archive moteur, header, CMakeCache et compile_commands exportés avant assertions d'identité, manifeste SHA propre à chaque variante. Le BUILD_ID du header doit être celui du moteur attendu avec profil exact, propre et présent dans l'ELF. Les options CMake utilisent les véritables noms BUILD_TESTS, ENABLE_ADDRESS_SANITIZER et ENABLE_THREAD_SANITIZER.

Chaque cas conserve argv/env/stdout natif intégral/XML/exit ; il doit émettre exactement son BUILD_ID natif, sans mélange des profils59/888, et exactement le cas nommé terminé sans skip/error. Le résultat rouge est extrait du XML après export, pas transformé en vert. Sur refus de compilation, le log et le code existent ; il peut n'y avoir aucun ELF final à exporter, aucune réussite de compilation ne sera déduite d'un fichier manquant.

Le gestionnaire tue le groupe privé même après sortie de son leader, le récolte et refuse tout résidu cgroup entre commandes. L'unité KillMode=control-group est arrêtée au besoin, puis MainPID/ControlGroup/PID attribués vérifiés. Après fermeture native, les manifestes déjà présents sont réempreintés, y compris sur résultat rouge partiel ; seuls les quatre verts complets exigent les deux manifestes. L'inventaire final hash les logs fermés. Premier refus, ledger et quiescence restent bruts ; absence de preuve ou expiration finale interdit tout vert. Pas de rootlog conçu pour transférer ce diagnostic comme qualification globale.

## Vérifications de préparation

[Reçu CPU](preuves/two-reds-preparation/pilot-cpu.json) : dix tests passés, dont identités absentes/mélangées, red/code/XML, scope vide/sauté/désactivé, quota, hash d'artefact partiel, premier rouge arrêté sur la boucle réelle du contrôleur, préflight refusé sans unité et minuscule Python expiré/récolté. Aucun C++/systemd réel. Le premier essai CPU avait9passés/1erreur de liste mock épuisée, corrigée dans le témoin ; le mécanisme d'arrêt n'a pas changé. Syntaxe Python lue par AST, bash -n et diff --check verts. Ces contrôles ne qualifient ni bwrap, ni la compilation, ni les quatre sondes natives.

Les deux tickets publiés du16nov (anomalies chaînes228 et plan lecteur) restent ouverts, comme append et C1. Aucun correctif de production proposé avant attribution. Demande de RO sur ce pilote exact et les ajouts d'identité, puis attente du GO distinct.
