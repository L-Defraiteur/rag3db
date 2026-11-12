## Actualisation prioritaire — 13 novembre2026

Première tranche symboles : source d00661e969eb7d77aa7d2f25c6cb5fa4f93f3308
qualifiée par passe complète : F5 5/5, minimal, lib2252/0/65 dont9navigation,
NAV40/0/0, lectures27/0/0, hôte13/0/0, PG privé36/0/0.
Les anciennes sections « passe en préparation » ci-dessous sont historiques.
Correctifs après le premier ciblé8/0/0 : DFS de rendu, compteur code20,
registre background privé de fixture ; RunScope/runtime et ses assertions inchangés.
Ticket de qualification vidange déplacé vers docs/tickets/closed ; ticket global
navigation ouvert pour intégration de la tranche puis grep/couverture étendue.

Preuves aux deux postes : ~/.cache/rag3weaver-build/
cx-symboles-d00661e96-reprise-20261112/ ; inventaire b43e9a0892602d5415f335016092fa75a9b5743b74a8492da0a8694a72a5716a.
3ELF observés/native29/223gabarits, Max32G, target8,63Go ; primitive
validation-yxtqqkgt VERT0/restes[]/nettoyé=true ; quiescence target[].
Budget INITIAL6000s conservé, durée cumulée4253,885s. 7879 rendu après santé
avant/après ; aucune pose du daemon personnel. Worktrees et base privée
cx_archi_symboles_20261112 conservés jusqu'à livraison/revue, dump puis nettoyage
avec garde courante et archives administratives fidèles. Pas encore de master.

# Archi — 12 novembre 2026

## État courant — lot symboles de la nuit

Ticket ouvert : `docs/rag3weaver-archi/tickets/2026-11-12-exposer-la-navigation-dans-les-symboles.md`.
WT `~/.cache/rag3weaver-build/cx-wt-symboles` aux deux postes, target unique PC
`~/.cache/rag3weaver-build/cx-archi-target`, lib29 complète ; pas de modèle/GPU.
Branche1 conserve tous les rouges et le vert ciblé30ab48674 ; branche2 repose sur
master741 avec sa revue préservée et qualification codeparsers a4bf. Diff de rebase
src/templates vide ; conflit documentaire fusionné sans perdre les deux sections.

Ciblé initial600 (montage, compilation, omission membres, inscription de capacité,
initialisation hôte et schéma du cas négatif) conservé intégralement. Étape corrective
DISTINCTE120 autorisée par orchestration :30ab48674 **8/0/0 en1,14s**, unité37,394s.
Vrai chat à modèle de fixture → outil déclaré → arbre → membres réels (run/typed),
pas de corps livré ; vrai relais atelier avec contexte backend et vue{} refusée.
Lecture sans policy.read_files refusée au chargement. Sept témoins purs : profondeur,
identités/homonymes/sources, fraîcheur, égalités, membres, budgets et compteur ≤2N².
ELF41bf0591… :223 gabarits inventoriés/aucun absent ; PID714177/start86738286 charge
native29 dans la scope privéeMax32 ; quiescence[] après. Preuves copiées au portable
sous les mêmes dossiers `cx-symboles-…-20261112`, rouge≠montage≠vert distingués.

Code commun : nouveau module+adaptateur dataflow, Plan déclaré borné, gabarits code/
atelier et inscription dans la capacité read_files ; aucun changement IR/parser.
Membres déclarés/propriétés restent des faits distincts des scopes de plages.
Qualification IR : docs/codeparsers/12-11-2026/navigation-ir/qualification.md.
Le ticket global reste ouvert (grep enrichi/exhaustivité/navigation plus riche non livrés).
Passe adaptée distincte en préparation : F5/minimal, liblarge, NAV, e2e lectures/hôte/
Postgres sur base privée ; source/recette publiées avant lourd, primitive commune,
premier rouge stop. Aucun master avant revue et tour explicite.

Les sections d'accord ci-dessous sont l'historique du lot livré (masteradcf01128527),
nettoyé aux deux postes avec garde fraîche/archives fidèles, preuves conservées.


## Lot autorisé en cours : accord de commande hors domaine

Ticket canonique :
`docs/tickets/closed/2026-11-11-un-accord-de-commande-hors-domaine-redemande-le-meme-accord.md`.
Autorisation/répartition désormais dans
`docs/tickets/closed/2026-11-12-paralleliser-les-suites-autorisees-sans-chevauchement.md` :
ce ticket clos atteste seulement l'affectation, pas la réalisation fonctionnelle.

ED possède commande.rs, run_nodes.rs et leurs gardes/tests backend. Archi ne modifie que
le témoin navigateur `src/daemon_host/chat_contextuel.rs` (et ses propres docs) ; aucun
changement runtime/React. Le script est réellement hors racine atelier ; le nominal intérieur
livré reste inchangé. Nouveau témoin :
`un_accord_exterieur_reprend_la_ligne_exacte_et_n_accorde_pas_sa_variante`.

Après clic réel sur la carte : reprise de la même ligne, vrai run en fond, vrai wait_output
sur sortie encore absente, signal libéré après annonce de l'attente, sortie et cwd réels
vérifiés. Ensuite même script avec un argument ajouté : nouvel accord requis, jamais donné,
aucun effet du mode variante. Ni décision ni résultat de commande simulés.

## Étape initiale — UI rouge après reprise de l'accord

- Rouge historique produit `b57313175` conservé aux deux postes dans
  `cx-ui-contextuel-b57313175-coupure-attente-20261111/` du cache de build.
- Nouveau témoin `5611fc98e`, après ticket `59a55ced3`, sur base master `6e5f9458c`.
  Premier montage : Chrome ferme CDP (0/1/0, 0,49 s), sans atteindre le clic.
- Le diagnostic instrumenté prouve le montage : stderr Chromium dit **Socket path too long**
  pour TMPDIR `cx-accord-exterieur-scratch`. Le premier diagnostic avait été retiré de la
  file avant lancement (NON JOUÉ), la mesure exclusive restant prioritaire.
- TMPDIR seul raccourci vers `~/.cache/rag3weaver-build/cx-ae` (disque). Même test et mêmes
  assertions, aucun déplacement du script. **Rouge produit 561 : 0/1/0 en 1,13 s** : vrai
  clic, deux lignes identiques, deuxième accord demandé au lieu de wait_output.
- Combinaison exacte ED `aed5ef1158093d1652531aa992878a1ad8612e29`, test vérifié identique :
  le clic relance la commande et atteint le vrai wait_output. Mais stdout reste vide ;
  **rouge 0/1/0 en 30,73 s**, attente de fin jamais satisfaite. Journal réel de commande :
  `python3: can't open file .../attendre-exterieur.py: [Errno 13] Permission denied`.
  La variante n'est pas encore atteinte. Aucun vert navigateur revendiqué.

ED a confirmé des rouges de portée et leur antériorité : l'ancien accord permanent ignorait
CetteFois. Sur aed, ED rapporte commande50/0 (helper enfant role_hote_qui_sort ignoré mais
joué par son parent), run_nodes18/0, incluant consommation unique/concurrence sans exécution
 destructrice. Relecture statique archi b4→aed sans objection concrète : jugement non consommateur,
verrou du choix de l'accord au mint, retrait avant retour si portée bornée CetteFois ; synchrone
et fond utilisent la même autorisation de ligne entière. Ce n'est pas une preuve du bac atelier.

La reprise logique fonctionne ; l'accès effectif au script extérieur reste à départager avec
ED/orchestration dans le même ticket et le contrat de capacités. **Ne pas déplacer le script,
élargir le bac, modifier les assertions/délais ou compter le ciblé backend sans sandbox comme
preuve du parcours navigateur.** Aucune passe globale ni master avant cette revue.

Preuves PC, copiées vers le portable :
`cx-accord-exterieur-5611fc98e-{baseline,chrome-diagnostic,chrome-diagnostic-2,rouge-produit}-20261112/`,
`cx-accord-exterieur-aed5ef115-vert-20261112/` (nom historique de tentative, résultat ROUGE).
Ce dernier conserve navigateur.log, source-head, exit-code, commande-reelle.stderr/stdout.
Les journaux de commande provenaient du run privé identifié par le chemin unique de fixture ;
seuls ces deux fichiers ont été copiés, aucun journal tiers modifié.

## Reprendre

Worktree isolé sur les deux postes : `~/.cache/rag3weaver-build/cx-wt-accord-exterieur` ;
branche actuelle `cx-archi-accord-exterieur-2`, base exacte aed sur PC ; branche1 garde la baseline561. Les
compléments locaux suivants sont documentaires. Target unique PC `cx-archi-target`, scratch
`cx-ae` (ancien scratch long conservé pour preuve), lib épinglée complète 29a7bc52a. Pas de base réelle ni service GPU.

Lire le refus d'accès réel avec ED/orchestration avant tout nouveau code ou replay. La source
ED f51c7f1f4 annoncée ne diffère de aed que par des docs ; elle ne répare pas ce dernier refus.
Le montage Chrome est départagé, aucun replay identique aveugle. ED possède le ticket canonique
et ses mises à jour ; archi n'y écrit pas concurremment. Nouveau ciblé seulement après source/
contrat annoncés et diagnostic précis. Le ticket reste ouvert.
Aucun nettoyage avant fin du lot. Le sous-module privé a été initialisé : son administration
est relevée dans le dossier baseline. Pour tout retrait utiliser garde_worktree.py courant,
archives administratives fidèles hors cible, rapport neuf et quiescence ; refuser en cas de
worktree dépendant ou doute. L'ancien script de nettoyage du lot UI du 11 ne convient pas.

## Résultat actualisé — paire déclarative rouge/verte prouvée

Orchestration a fixé explicitement la précondition avant modification : déclaration
`workspace.sandbox.extra_read` du seul fichier script extérieur, sans ouvrir son parent.
Le ticket/contrat ED 34f1ea8fb précède la fixture 996af8ee4. Cette dernière ajoute uniquement
un helper de configuration dans les tests atelier_fichiers et adapte le nouveau témoin ;
le nominal reste inchangé. Les droits chargés sont vérifiés : singleton exact en lecture,
aucun extra_write. Un accord de ligne ne crée aucun droit noyau implicite.

Avec exactement les mêmes fichiers de test/helper :

- Ancien runtime, source 8b94990067e5923dac287a4332e5ce0c0eca150f : rouge 0/1/0 en 0,90 s,
  après clic réel, deuxième accord demandé pour la même ligne.
- Runtime corrigé aed, source 996af8ee48148eda0e2b053686b0a3676554bfbb : vert 1/0/0 en
  1,32 s. Reprise exacte, vrai wait_output, sortie/cwd/page vérifiés ; variante du même
  script avec argument supplémentaire soumise à un nouvel accord, sans effet.

Attestation du binaire réellement testé : 220 gabarits, aucun absent ; SHA256 ELF
`0ec7bf3d32e3f92c82b756e44903451e4cddd3313b0b45bc6b9eaef31b910c50`.
Preuves complètes aux deux postes, hors target :
`~/.cache/rag3weaver-build/cx-accord-exterieur-8b9499006-rouge-declare-20261112/` et
`~/.cache/rag3weaver-build/cx-accord-exterieur-996af8ee4-vert-declare-20261112/`.
Les échecs initiaux sans extra_read sont conservés et ne sont pas requalifiés en verts.
Le diagnostic causal ED 157a2f272 est distinct : bac actif, refus sans déclaration,
succès avec le fichier seul, frère encore refusé (ED rapporte 1/0/0).

ED a repris 996 à l'identique dans son candidat groupé et a lancé sa globale après revue
et autorisation parent. Archi ne lance pas de globale et ne publie pas sur master.
Les passages « à départager » ci-dessus décrivent l'étape antérieure, désormais résolue
par cette précondition déclarée ; le résultat reste limité à cette configuration.
Le ticket canonique et le runtime restent à ED. Conserver sources et preuves jusqu'au
verdict groupé ; tout nettoyage ultérieur exige la garde et les archives administratives.

## Raccord ED final

Verdict composé sourceb42 confirmé par ED : features5/native1609/large2176/nav40 et
lectures24+démon13. Les deux e2e ont une preuve réelle de PID/cgroup/Max32/KillMode ;
les droits noyau restent déclarés séparément. La fermeture fonctionnelle accompagne le
lot ED ; le nettoyage des ressources archi est consigné par leur propriétaire, pas présumé ici.

## Nettoyage archi attesté

Archi confirme le retrait aux deux postes de son worktree cx-wt-accord-exterieur, de son
target cx-archi-target, des scratches et de ses trois branches locales, après quiescence,
archives administratives fidèles (sous-module PC inclus) et garde courante dca13abb… verte.
Remotes et preuves conservés dans cx-accord-exterieur-nettoyage-garde-20261112 ; copies
PC et portable distinctes. Une branche PC sans upstream a d'abord refusé -d ; son upstream
distant exact a été vérifié et renseigné avant le -d normal, sans -D. Cette attestation
vient d'archi ; aucun nettoyage ED n'y est assimilé.
