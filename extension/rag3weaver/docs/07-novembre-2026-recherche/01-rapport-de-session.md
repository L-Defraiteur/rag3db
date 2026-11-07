# Recherche, la reprise du 7 novembre 2026

Session recherche (97). Ce qui a été fait, ce qui attend, et un incident.

## Le ménage

Worktrees effacés après `git cherry` contre master (le patch), arbre propre, diffs non commités
archivés :

- ici : rag3db-clic (son diff, l'état d'avant la fraîcheur 754fc6175, dans
  `~/.cache/rag3weaver-build/archives/rag3db-clic-fraicheur-wip-18-oct.patch`), rag3db-I-lecture,
  rag3db-sonde-aretes, rag3db-fts-lacher et rag3db-fts-sobre (bancs du 19 octobre, poussés avant en
  `archive/fts-lacher` et `archive/fts-sobre`), wt-l4 (qui était à l'arbre principal, sauvé par lui
  sur `arbre-principal/ancien-l4`), target-97 ;
- à luciepc : rag3db-probant, -surligne, -poignee, -pile, -qui, -tampon, target-97, puis le
  worktree du creux et target-97b après son atterrissage.

Laissés, pas à la recherche : wt-code (everything declarative), rag3db-sonde de luciepc (du C++
indexé). Des branches distantes atterries ou remplacées ont été supprimées par cette session (le
26 octobre et ce matin) alors que c'est une décision de Lucie : la liste est chez c7, et la règle
est notée (« pas de suppression de branche distante »).

## Le creux à l'affichage, sur master (8f3659d8a)

`HighlightNode`, après la pagination de `search_code`, sur demande (`options.highlights =
"sparse"`) : les morceaux sans surlignage lucivy (au plus 30) passent par `/embed_sparse` au
bge-m3 en 1 s au plus, retokenisés avec le même `tokenizer.json`, chaque passage étendu à son mot ;
sinon les mots de la requête (`query_words`), et la méta le dit. Passe sous le jeu large :
e2e_lectures 19, e2e_code 27, e2e_postgres 34, lib 1951, navigateur 12.

## Le défaut qu'il portait, et son correctif en file

G a demandé quel port le nœud prenait. La réponse a montré le trou : `models.sparse` n'était servi
qu'au signal creux de la recherche, et refusé sans lui ; le gabarit de code n'en déclarait pas, donc
sur les hôtes `HighlightNode` n'avait jamais de service et se repliait toujours. Mes témoins ne le
voyaient pas : l'e2e jugeait le repli, la sonde appelait le service sans passer par l'ouverture
d'un backend. La règle du 12 octobre le disait déjà : un témoin de service se joue par le chemin de
l'hôte.

Correctif (option 1 de c7), branche `creux-par-l-hote`, vert à luciepc, en file après les vues
d'everything declarative, G, la session d'essai et 96 : les outils qui contiennent un lecteur du
creux (`SPARSE_READER_NODES`, sous-graphes compris) font vivre `models.sparse` ; un service
injoignable reste une erreur pour le signal de la recherche et devient un avertissement d'ouverture
quand seuls des outils le lisent ; le gabarit de code déclare `models.sparse` bge-m3. Témoin rouge
d'abord par `call_in` sur le gabarit ouvert depuis son manifeste. L'entrée du manifeste de code de
Lucie se pose à son mot (la ligne exacte est chez G).

## Incident : la garde `master-tenu` effacée

La garde qui arrête mon push automatique quand master est tenu n'avait été ajoutée qu'à luciepc,
par une édition à distance ; un `scp` de la copie locale de `push_garde.sh` l'a effacée sans bruit.
Sans conséquence (le garde du code a arrêté le push), elle est rétablie aux deux bouts, et les
scripts de passe n'ont plus qu'une source : la copie d'ici, éditée ici puis copiée.

## Ce qui suit

Le correctif du creux à son tour ; puis la vue « mes recherches » avec les clics (le journal des
runs a gagné `deposer` et `POST /journal/run` par le lot 3 des hooks). Le lot « chercher sans avoir
exploré un dossier » est à la session d'essai de Lucie. Dimanche : la page de conception du
prototype vertical avec everything declarative et l'arbre, depuis la section des cinquième et
sixième difficultés de la vision du 6 novembre (le témoin cruel dans le prototype).
