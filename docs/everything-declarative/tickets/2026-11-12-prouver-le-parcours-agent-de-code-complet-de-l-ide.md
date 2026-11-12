# Prouver le parcours agent de code complet de l'IDE

- **État** : ouvert ; lot isolé autorisé pour la nuit après les correctifs des accords.
- **Famille** : produit
- **Gravité** : capacité principale ; l'atelier doit réellement explorer, modifier, tester et reprendre.
- **Atteignable en service** : les outils existent ; leur parcours complet dans une session réelle reste à qualifier.
- **Touche rag3weaver** : oui, chat/agent de l'atelier, outils accessibles, exécution et reprise.
- **Pour** : ED ; archi relit les contrats d'interface, sans édition concurrente.
- **Signalé par** : objectif explicite de Lucie pour l'IDE et demande de lots indépendants cette nuit.
- **Ouvert le** : 12 novembre 2026

[Répartition](../../orchestration/12-11-2026/02-lots-independants-pour-la-nuit.md).
Créer un hôte, un dossier d'état et un petit projet Git de fixture indépendants
de l'IDE personnel. Passer par le vrai chat et le catalogue d'outils du framework :
demande synthétique, recherche/lecture, modification, vrai test nommé, résultat,
diff et reprise. Utiliser Gemini via Vertex configuré quand le modèle est requis,
avec une recette et des appels bornés ; aucun secret ou chat réel envoyé.

Vérifier fichiers et effets réels, nombre/tests effectivement exécutés, travail
isolé et continuité des identités de conversation/outils. Une réussite verbale,
une fonction Rust présente sans binding ou un appel direct contournant le chat
ne satisfait pas le contrat. Les refus et accords suivent le mécanisme normal,
sans mode permissif de test masquant un défaut d'accessibilité.

Publier le premier constat/source/recette avant ciblé. Corriger les lacunes dans
les primitives et gabarits communs ; respecter propriété navigation/code_tools
et modèle/serveur des autres sessions. Pas de nouvel orchestrateur multiagent
builtin, ni de déploiement/restart personnel dans ce lot initial.

Fin : parcours réel reproductible avec outils accessibles, tests/diff/reprise
observés, corrections éventuelles relues et intégrées, limites restantes dites.
Le ticket reste ouvert si seule une partie du parcours est démontrée.

## Revue du correctif de lecture Git b0090e2be

Les preuves36e restent partielles : vrai chat Vertex, exploration, test nommé
rouge puis édition et test vert ; diff refusé128, reprise pas encore atteinte.
Le helper proposé réduit les chemins communs et conserve les écritures.

Contre-exemple concret à qualifier avant nouvel avis favorable : créer une
racine faux avec son entrée `.git` **lien symbolique vers le gitfile d'un vrai
worktree**, puis appeler avec_lecture_git sur faux et le principal du dépôt.
La comparaison actuelle canonicalise les deux gitfiles : retour enregistré et
gitfile faux deviennent le même chemin réel. Le contrôle de retour passe alors
pour une racine qui n'est pas le worktree enregistré ; son administration
(éventuellement celle d'un voisin) est ajoutée aux lectures.

Revue statique, aucun scénario de ce type exécuté par orchestration. Ajouter le
témoin négatif avec la même exigence que le gitfile copié déjà refusé ; valider
la racine de worktree enregistrée, pas seulement l'égalité après résolution du
lien final, ou refuser explicitement ce gitfile symbolique non qualifié. Préserver
le dépôt ordinaire, le worktree lié réel et ses espaces ; aucune écriture du
principal/voisin ne devient nécessaire. ED garde le correctif et sa qualification
kernel puis chat sous source figée ; cette revue ne remplace pas ces résultats.
## Reprise ED et premier constat

Worktree isolé `cx-ed-parcours-agent-ide`, branche homonyme, base `a783013f1`.
Aucun appel modèle, build ou service lancé à ce stade. Les fichiers réservés
navigation et résidence des modèles restent à leurs propriétaires.

Le témoin existant `le_chat_d_une_tache_code_sur_gemini` utilise bien le vrai
chat et Vertex mais ne qualifie que l’édition. Le gabarit code déclare déjà
la chaîne Rust en lecture et Cargo hors réseau dans son bac à sable ; les
méthodes normales relisent tâches et conversations persistées.

Préparation : fixture Git Cargo sans dépendances, test nommé immuable,
exploration par outils exposés, échec initial attendu du test, correction et
réussite effective, diff par commande autorisée puis recréation de l’hôte et
reprise. L’attestation indépendante vérifiera les octets du principal, les
identités et les appels/résultats liés, sans remplacer les actions de l’agent.
La source et la recette bornée seront publiées avant compilation/ciblé.

## Première source et recette bornée

Nouveau témoin ignoré par défaut :
`daemon_host::chat_qui_code::le_chat_code_teste_et_reprend_sur_vertex`.
Il emploie le modèle réel par défaut de l’hôte, sans callback substitué.
Trois tours : recherche/lecture et test initial attendu rouge ; édition du
seul fichier source puis test vert et diff ; nouvel hôte, registres relus,
historique identique (identifiants outils inclus), test et diff rejoués.
Le test d’intégration immuable `salutation_nettoie_le_nom` doit effectivement
s’exécuter (1 failed attendu puis 1 passed), jamais zéro test. Les preuves JSON
et la fixture sont conservées sous un parent privé, y compris en cas de refus.

Sur luciepc uniquement, target neuf propre à ce lot, lib native29 épinglée,
j8/Max32G/HIGH0, aucune charge GPU nécessaire et aucun usage 7879 prévu.
Primitive commune extérieure, budget global1900s incluant nettoyage :
compilation lib de tests1500s maximum, puis témoin exact360s maximum.
Features initiales `rag3db-native,code,daemon,openai-llm` ; pas de navigateur
à ce préfixe (preuve du contrat hôte/chat/outils, pas encore preuve visuelle).
Compilation par `cargo test --lib --no-run --message-format=json` ; extraction
de l’unique ELF lib dans le flux JSON. Sonde `--list --ignored --exact` sur
le nom complet, puis ELF `--ignored --exact <nom> --nocapture --test-threads=1`.
Attendu : exactement1 témoin, 1 passed/0 failed/0 ignored. Au premier rouge
inattendu : arrêt, preuve et diagnostic avant toute nouvelle recette.

Variables runtime : `CARGO_MANIFEST_DIR` du worktree ; `LD_LIBRARY_PATH` lib29 ;
`RAG3WEAVER_EXIGER_GEMINI=1`, chemin autorisé des identifiants Vertex (jamais lu),
projet configuré `lr-hub-472010`. États, journaux, targets de fixture et preuves
sont isolés sous le dossier privé du lot. Le target Cargo de la fixture est
celui du contrat sandbox normal, sous `RAG3WEAVER_TARGETS_DIR` privé. Aucune
politique permissive ou réponse à un accord humain n’est injectée.

Préflight statique avant tout lourd : le gabarit livré active le rattrapage
vectoriel à l’ouverture. La déclaration de cette fixture non indexée le désactive
explicitement ; `search_code` garde son balayage déclaré. Aucun modèle substitué,
ni charge d’indexation nécessaire au parcours. Cette source complète remplace la
première pour le ciblé, encore non lancé.

## Rouge réel initial conservé et correction préparée

Source `36e284824` : compilation verte134s, unique témoin rouge56.77s.
Exploration par balayage, lectures, test nommé1 failed attendu, édition et test
nommé1 passed effectifs. `git diff -- src/lib.rs` rend code128 : Landlock refuse
l’administration externe indiquée par le gitfile du worktree. La reprise n’a
pas été atteinte. Preuves PC : `parcours.log`, JSON01/02 et supervision
`validation-3_5jj1d5` : rouge101, `restes=[]`, `nettoye=true`. Aucun rejouage.

Correction commune proposée : `backend_code::avec_lecture_git`, raccordée au
montage du bac dans `backend.rs` (hors initialisation modèles réservée). Elle
compare le répertoire commun canonique avec celui du principal porté par l’hôte ;
un worktree doit être enfant direct de `common/worktrees`, avec retour `gitdir`
vers son propre gitfile. Lectures : son administration seule, puis les chemins
communs nécessaires `objects`, `refs`, `HEAD`, `config`, `packed-refs`, `shallow`.
Pas de dossier commun entier, pas de racine issue d’un lien symbolique sortant,
pas de configuration utilisateur ouverte et aucune nouvelle écriture accordée.
Les objets alternatifs extérieurs restent soumis au refus du noyau.

Témoin kernel nommé
`backend_code::tests::le_bac_git_lit_le_diff_sans_ouvrir_les_voisins` : dépôt
ordinaire et worktree lié avec espace ; diff réel ; lecture du principal privé
et HEAD du worktree voisin refusées ; copie dans le domaine permise, vers
principal/config Git/HEAD voisin refusée ; gitfile forgé, dépôt étranger et
symlink sortant ne créent aucune autorisation. Ce test de la primitive noyau
ne fabrique aucun accord dans le parcours agent.

Relecture Archi retenue : résultats JSON décodés et liés au `tool_call_id`, à
`arguments.command` exact et au code réel ; phases séparées pour ne pas relire
le test initial ; lectures des deux chemins et immutabilité tests/Cargo dès01.

Recette corrective DISTINCTE, pas remplacement du rouge initial : même target,
lib29 et conditions ; global1200s incluant nettoyage, compilation600s,
témoin kernel60s, témoin réel360s. Les preuves correctives vont sous
`parcours-agent-ide-20261112/correctif-git` ; aucun nouvel appel Vertex avant
publication de cette source et relecture du delta de permissions. Attendu
kernel1/0/0 puis réel1/0/0 ; arrêt au premier rouge inattendu.

La lecture des objets et références expose l’histoire du dépôt, pas seulement
les lignes du diff courant ; cette capacité reste en lecture et porte sur le
même dépôt connu de l’hôte. Le fichier de configuration global privé demeure
inaccessible. Si Git réclame une actualisation d’index facultative, désactiver
ses verrous facultatifs, jamais ouvrir l’administration du principal en écriture.
Source corrective figée `b0090e2be` : compilation/kernel lancés, attente d’un
avis RO sur cette source avant l’étape Vertex, dans le même budget1200s.

Correctif `b0090e2be` : compilation13.94s verte ; témoin noyau rouge0.03s
sur `git diff` code128, après lecture de l’administration désormais possible :
Git charge implicitement `~/.gitconfig`, inaccessible, puis rend une erreur
fatale de configuration. Aucun Vertex correctif appelé. Supervision rouge101,
restes vides et nettoyage vrai, preuves `correctif-git/kernel.log` conservées.

Delta motivé : par défaut dans le bac, `GIT_CONFIG_GLOBAL=/dev/null` et
`GIT_OPTIONAL_LOCKS=0`, sauf valeur explicitement déclarée dans `sandbox.env`.
La configuration du dépôt reste lue ; la configuration personnelle n’est ni
ouverte ni copiée. Aucun droit ajouté. Nouvelle étape corrective2 distincte,
même recette1200s/compile600/kernel60/réel360, preuves `correctif-git-2` ;
Vertex attend toujours la revue de cette nouvelle source.

Source `b310b5c0c` : compilation puis noyau1/0/0 vert0.10s. Aucun Vertex :
Archi relève un alias INTERNE encore accepté (`refs` vers
`common/worktrees/voisin`). La portée en attente a été arrêtée volontairement
par pidfd du superviseur1164394/start87036829, identité et argv vérifiés dans
son reçu privé, avant toute ouverture de la gate modèle. Ne pas confondre ce
vert partiel avec le parcours complet.

Correction de la réserve : chaque chemin commun accordé doit être exactement
le chemin canonique attendu, pas seulement un descendant du commun ; les alias
internes et externes n’ajoutent donc aucune racine. Témoin supplémentaire :
`refs` vers administration voisine et `config` vers son HEAD ; lecture noyau du
HEAD voisin toujours refusée. La configuration explicitement déclarée conserve
sa priorité sur le défaut isolé. Étape corrective3 distincte même recette1200s
et target, preuves `correctif-git-3`, attente de revue avant Vertex.

Revue C7 `65b57052f` : autre contre-exemple statique, faux `.git` symbolique
vers le gitfile réel, dont la canonicalisation confondait les occurrences.
Réserve retenue avant tout Vertex correctif. Le gitfile final symbolique est
maintenant refusé avant découverte des métadonnées ; témoin négatif ajouté
au côté du gitfile copié. La portée corrective3, noyau1/0/0 vert0.12s, est
arrêtée volontairement par pidfd du superviseur attribué avant la gate.
Nouvelle source corrective4 à relire, mêmes bornes1200/600/60/360 et target,
preuves `correctif-git-4` distinctes ; aucun appel modèle pour les étapes2/3.
