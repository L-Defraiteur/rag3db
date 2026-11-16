# Paquet core adapté pour relecture — pas de GO d’exécution

Ticket porteur : `docs/lucivy/tickets/2026-11-16-indexed-false-est-ignore-pour-un-champ-texte.md`.
Limites persistantes : `2026-11-16-le-profil-core-par-defaut-inclut-des-diagnostics-non-isoles.md`.
Cette recette remplace la proposition07 pour le prochain GO éventuel.

## Couple et paquet exacts

APP `890c86af44989f3fea6c6fa62fd40d688b4084df`, descendant du main audité
`a1c0dfddfb97666aee148480f8eb3e98d00885ce`. Pas de rebase nécessaire.
W PC `/home/lucied/git_workspaces/cx-lucivy-indexed-false-warning-16nov`.
Q `/home/lucied/.cache/rag3weaver-build/cx-lucivy-indexed-warning-core-16nov-v1`,
nonce neuf `e14c4d52ea84cbb255feeb63679cfcb3`, UID/dev/inode dans
[précontrôle](preuves/indexed-warning-core-preparation/preparation-check.json).
Scope et logs encore absents ; scratch privé vide. Aucun Cargo ou ELF exécuté.

Pilote `outils/qualify-indexed-false-core.py` SHA256
`2fa59faafe18634a055da4e770a96d4eb87da79864332f8d2a5d8ed222a8fa14` ;
admission `outils/admit-indexed-false-core.py` SHA256
`3cde2f1ebd9a038641c403e75d5a4eb425ba25fe34cf0d2b0c62361101d707a2`.
PréflightSHA `23dd351692d45004e23f5a92c3d9ec6d5706cb03008a6afee3bc816c6aa4b3be`.
Copies figées : contrôlesrouges0a269dd90b0ca745811713e21f66b45fda563074bfc06dd6993f04ab03bd2352,
Common6439fdd8b52c85fa4e74af0def2c667ef0bedae9b814cbbfd48e49946c4f74ea,
poste79e3a8bbf457e3bc9104aa1558e7d03072216c70b68e08ae606ace6aa4584c7b mode755.
Le manifeste complet des copies figure dans le preflight archivé.

Même T unique `/home/lucied/.cache/rag3weaver-build/cx-lucivy-target-16nov`,
noncecréateur3552903efab590b44f44cd8002f7b028, UID1000/dev55/inode164536077,
markerSHA5a1671d22dc784bec78f60cf25c8b3003acc5f21e28290f05d0dfc22e0c394df.
ELFprécédentvert d10152c78102ee701ef2b808fc2627d7e8044866fcbc5cebe9e5a23550958adc
copié RO dans Q/green-test-elf et rehashé, original T identique. Reçu du Commonvert
1575006/start118237703 TERMINE, verdict0/restes[]/nettoyé ; enfants owned absents.
Deux relevés partiels de préparation conservés : **12 puis13 UNKNOWN** nonattribués,
aucun owned/attribué bloquant, pas de quiescence universelle revendiquée.
Les contrôles seront frais après acquisition du vrai poste puis après le run.

## Corpus réellement préparés sous admission PREPA de C7

Préparation seule bornée180s, terminée en2,25s ; outil exact conservé dans
`outils/prepare-indexed-warning-corpora.py`, SHA
3c55ddb9bca00d0cc565fd4043091c071761bd02d08fbb0a6439e3f80a7fdb48.
Aucun test, build, base ou service. Les fichiers des copies sont RO et les
racines privées mode700. Archive originale hashée avant/après identique ;
référence HEAD du clone inchangée. Aucun contenu .env/.vault lu.

- Linux **2.6.0**, 15007 fichiers réguliers, extraction sûre dans la racine
  réelle privée `/tmp/lucivy-cmp`, initialement absente. Pas de symlink.
  Archive publique cache4d0299b3f9a5149635173998fc104c88c533523a2b43098a95b4ceba72dc6f73,
  origine kernel.org déclarée. Licence GPL du noyau conservée intégralement
  dans COPYING ; empreinte et texte archivés. Manifeste exhaustif
  d4faf3a7988f42db9d0801ee5bd6630d01f97f7be58ce9838fb6cbe189d4f204.
- rag3db commit **1b22409b31a79543a9fe63e516c5b311bc43f1da** : 8507 blobs
  réguliers copiés depuis les objets Git du cache public, sans lecture du
  contenu de son working tree. Racine privée Q/corpora/rag3db, RAG3DB_ROOT et
  V3_CORPUS pointent exactement dessus. .env.example exclu AVANT lecture,
  gitlink codeparsers et symlink tools/rust_api/rag3db-src exclus ; liste complète
  dans le manifeste. Pas de .git dans la copie. Licence LRSL1.2 conservée,
  origine GitHub et hash de LICENSE archivés. Manifeste exhaustif
  bb70899c1fba6cac19f58a735bcc2e56a0e25de00dfd33f13a4785b08f608e12.

Une vérification RO après copie a rehashé les23514 fichiers et contrôlé
l’absence de liens/fichiers supplémentaires, en1,14s ; aucun Cargo.
[Manifeste et preuves](preuves/indexed-warning-core-preparation/SHA256SUMS).

L’audit des tests dictionary_index, derived_in_ram et positions_off montre
qu’ils comparent les régimes sur le même corpus réel : extensions c/h/rst/txt,
taille1..60000, UTF8, ordre DFS trié, au moins50. Pas de nombre attendu lié à
une version du noyau ni de golden7.2 substitué. Les listes exactes des300 et400
premiers fichiers consommables sont publiées ; 12133 fichiers admissibles.
Aucune assertion de contenu ou de résultat assouplie : si le panel2.6 ne suffit
pas, le test sera rouge. La mention7.2 du test missing_corpus vérifie le texte de
la recette d’absence ; elle ne charge pas un étalon7.2.

Ground-truth V3 et merge_corpus utilisent la copie rag3db déclarée, avec leurs
limites par défaut ; pas de V3_MAX_DOCS/V3_QUERIES ou filtre hérité. Le fichier
par défaut test/c_api/query_result_test.cpp du diagnostic fuzzy est présent.
Le cache ground-truth porte le chemin privé complet (donc le nonce dans son nom)
comme clé ; cache initialement absent, absence recontrôlée par le préflight,
aucun golden existant adopté ou invalidé. Les fixtures3.0.8 embarquées restent
les fixtures du candidat ; pas de remplacement par ce corpus.

**Linux7.2 et l’ULP restent NON JOUÉS/HOLD**. L’APP de ce lot ne contient pas le
WIP parse-sum-order1b9f47d ; aucune équivalence de corpus, de calibration ou de
snapshot n’est revendiquée.

## Profil explicite et neuf trous de couverture

Compiler lib+tests du core par défaut (release/offline), inventorier TOUS les
ELF produits, leur SHA, `--list` et `--list --ignored`, puis jouer sans filtre
les targets admis. Doctests séparés sous la même échéance. Les trois ciblés
warnings sont inclus dans ce contexte élargi, pas rejoués seuls sans cause.

Ces neuf targets sont **HOLD, aucun test exécuté** ; inventaires toujours
produits et exposés, pas de `--skip` caché ni de test peint en vert :

| Target | Prérequis non satisfait / raison |
| --- | --- |
| bench_vs_tantivy | Chemins personnels persistants, réemploi d’index et suppression de verrous |
| test_store_fallback | Diagnostic sur index personnel persistant |
| test_diagnostics | Diagnostic sur index personnel persistant |
| test_lock_investigation | Diagnostic sur index personnel persistant |
| test_search_mismatch | Diagnostic sur index personnel persistant |
| test_luce_roundtrip | Snapshot exact playground/dataset.luce absent ; ne pas lui substituer2.6 |
| test_fuzzy_ground_truth | Parcours du checkout n’excluant pas .vault |
| test_regex_ground_truth | Parcours du checkout n’excluant pas .vault |
| test_merge_contains | Parcours du checkout n’excluant pas .vault |

Les autres `#[ignore]` restent également nonqualifiés, particulièrement les
services PostgreSQL, diagnostics et mesures manuelles. Inventaire source
préliminaire regex :283 annotations test/43ignore (ce n’est PAS un inventaire
ELF ni un compte exact de cfg). Les comptes faisant foi viendront de libtest.
Les retours d’absence ne sont pas des réussites ; tout autre message de skip
rencontré devra être relu avant verdict. Le résultat porte explicitement
`full_core_qualified=false`, indépendamment du code0 du superviseur sur le
profil adapté. Aucune affirmation « core entier vert » possible avec ces trous.

## Commande proposée — NON lancée, RO puis GO distinct requis

```bash
POSTE_MEM_MAX=32G RAG3WEAVER_BUILD_MEMORY_HIGH=0 /home/lucied/.cache/rag3weaver-build/cx-lucivy-indexed-warning-core-16nov-v1/support/poste lourd timeout --signal=TERM --kill-after=5s 1810 python3 /home/lucied/.cache/rag3weaver-build/cx-lucivy-indexed-warning-core-16nov-v1/admit-indexed-false-core.py
```

Après acquisition : START monotone unique, END=START+1800s. Chemins sans liens,
UID/dev/inode/nonce, hashes des copies/ELF/marker, ancienne autoritéTERMINE et
scanpartiel frais ; confineur pour le seul reste moins2s, aucun reset. Mémoire
32G/HIGHmax relue dans le cgroup, jobs min4/affmoins2/quota hiérarchique,
libtest2threads. Arbre source890 propre ; corpus exhaustivement rehashés avant
compilation, nouvelles options de corpus/build refusées. Scratch vide à
l’entrée et à la sortie ; rapports/cache V3 neufs sous le target détenu.
Premier processus nonnul : arrêt et first-red, pas de retry. Inventaires,
résumés/ignored/HOLD, logs doctest, SHA de chaque nouvel ELF et scans archivés.
Résultat du pilote provisoire : seul Common terminé sans restes et scan final
owned absent autorise un verdict du profil borné. Pas de main/tag/publication,
PR42 reste refus/HOLD, aucun contournement.

Préparation vérifiée : AST valide et trois simulations pures du classificateur
3/0, pas de /proc réel dans ces simulations. Un premier harnais temporaire
avait omis l’alias classify (troisNameError), corrigé sans changer le pilote.
Un précontrôle de cache avait exigé à tort son parent déjà présent ; corrigé
pour contrôler les ancêtres existants et garder l’absence comme prérequis,
sans créer de cache/rapport ni modifierT. Les contrôles finaux sont ceux archivés.
