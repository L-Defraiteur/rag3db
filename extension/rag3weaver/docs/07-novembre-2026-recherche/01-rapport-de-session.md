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

## Le soir : trois lots en un passage, et le ménage qui suit

Trois lots verts réunis sur `recherche-trois-lots` (une passe, un push à mon tour) : le creux servi
par l'hôte (`models.sparse` lu par `HighlightNode`, 0 appel à l'indexation), le filtre du signal
creux juste (les offsets des morceaux des parents admis), la requête longue bornée (L = 100
caractères de flou, K = 64 termes, tirés de la mesure portée au ticket). Une mesure a d'abord été
mise sous `poste mesure` à luciepc et a gelé la file des lourds : retirée sur le mot de c7,
rejouée sous `poste lourd`, la charge notée.

Effacé ce soir, en le disant (règle de Lucie relayée par c7) : ici, les worktrees
rag3db-creux-hote, rag3db-filtre-creux et rag3db-borne (≈ 1,3 Gio) et la copie de mesure de la
mémoire longue (33 Mio) ; à luciepc, les mêmes worktrees (≈ 1,3 Gio), `target-97` (6,8 Gio) et les
copies de mesure de code-principal et de la mémoire longue (1,6 Gio et 33 Mio) — df de luciepc
584 → 587 Gio. Les branches distantes restent pour la décision de Lucie.

## La requête entière longue dépend d'un correctif de lucivy

lucivy a relevé ce soir qu'un passage exact d'un document n'est plus trouvé au-delà de 48 octets
en strict et de 64 en relâché, à un comme à quatre shards (ticket lucivy « un passage exact long
n'est plus trouvé », branche `tickets-2026-11-07`, sonde `probe_exact_passage_lengths`). La borne
de la requête longue (77d0b0676) envoie la requête entière en strict au-delà de 100 caractères :
elle tombe exactement là où lucivy ne trouve plus rien. Le témoin le montre chez nous, par le
chemin de l'hôte — `e2e_lectures::une_ligne_collee_de_120_caracteres_retrouve_son_fichier`
(branche `passage-exact-long`, d75b73466) : une ligne de 115 caractères collée telle quelle ramène
`src/sources.md` premier par le vecteur et les mots, sans `bm25_exact`, au lieu de son fichier. Il
est rouge tant que lucivy ne corrige pas, et c'est lui qui jugera la montée de version. La borne du
coût reste juste (le coût du flou est corrigé à part dans lucivy, 9356b58, pas encore publié) ; ce
qui manque est la trouvaille d'un passage long, pas la vitesse.

## Copier une base pour la mesurer : base et plein texte ensemble, à l'arrêt

La seconde mesure de la mémoire longue (sur lucivy 4.3.2) a vu un index plein texte de 0 document,
alors que la base en service en a 2 (vérifié par G : « cartes poste », « hooks essai »). Le 0 venait
de la copie : `memoire-longue.rag3db`, son `.wal` et son dossier `.fts` copiés par reflink **pendant
que l'hôte tenait la base** (sa marque `.ecrivain` posée, le WAL écrit quelques minutes avant). La
base et le plein texte ont pu être pris à deux générations différentes ; un dossier `.fts` dont la
génération n'est pas celle de la base est reconnu « à rebâtir » à l'ouverture, et la copie l'a vu
vide. La première copie, à 17 h 17, était tombée juste par chance.

**La règle** : une base se copie pour mesure avec tout ce qui la compose — `.rag3db`, `.wal`,
`.extensions` et le dossier `.fts` — **en un même instant sans écrivain** : l'hôte arrêté pour ce
backend, ou un instantané pris par l'hôte (G) ; jamais pièce par pièce sous un écrivain vivant. Avant
de mesurer, lire le nombre de documents de la cible dans la sortie de la sonde et le comparer à la
base en service : un index vide ou plus petit veut dire que la copie est fausse, pas la mesure.
