# La garde du push n'a pas de mode « tests seuls »

- **État** : fait (16 novembre 2026, branche guard-tests-only) : sous TESTS_ONLY=1, push_garde.sh laisse passer sans lib un lot qui ne touche que les dossiers de tools/poste/tests_only_dirs.txt (tools/shell/test/), plus des .md, sur un journal qui dit « tested binary: <chemin> commit <40 hex> sha256 <64 hex> » ; tout autre chemin, ou un journal sans provenance, est un ARRÊT nommé ; trois cas neufs au témoin, 3 rouges sur la garde de master
- **Famille** : produit
- **Gravité** : un lot qui ne touche que des tests Python du shell ne peut pas atterrir seul. `push_garde.sh` exige
  qu'un journal de la passe nomme une lib du magasin (« zéro reste un ARRÊT », l. 188-192). Or un tel lot n'en a
  pas : sa preuve est la suite du shell, jouée contre un binaire du shell. Il doit donc attendre d'être empilé sur
  un lot Rust.
- **Atteignable en service** : non (outillage du push)
- **Touche rag3weaver** : non (`tools/poste/push_garde.sh`)
- **Pour** : G (après ses trois lots)

## Ce qu'on voit

Le 16 nov., le lot `shell-tests-own-history` (b2ac66efd, rien d'autre que `tools/shell/test/conftest.py`) était
vert : suite du shell 245/245, témoin rouge puis vert. Il n'a pas pu passer la garde, faute de lib. Le lot
précédent du shell n'est passé que parce qu'il était empilé sur la passe du journal des runs. Faire nommer une lib à
un journal pour franchir la garde tromperait son contrôle.

## Pour fermer

Un mode « tests seuls », pendant du mode docs :

1. Le diff du lot reste sous des dossiers de tests déclarés (`tools/shell/test` pour commencer). Tout autre chemin
   est un ARRÊT.
2. Il n'exige pas de lib. Il exige un journal de la suite (le 245/245) qui nomme le binaire joué, avec sa provenance
   (son chemin, son commit, son empreinte), et une dernière ligne VERT sur le commit poussé.
3. Les trois contrôles restent les mêmes.
4. Un témoin de script : un lot de tests seuls passe, un lot qui ajoute un fichier hors des dossiers déclarés est
   refusé.

En attendant, un lot de tests seuls s'empile sur le prochain lot qui a une passe avec sa lib.
