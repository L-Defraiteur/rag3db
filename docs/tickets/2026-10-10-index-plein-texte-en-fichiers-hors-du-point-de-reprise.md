# L'index plein texte en fichiers n'est pas tenu par le point de reprise du moteur

- **État** : ouvert — décision de Lucie, 10 octobre 2026 au soir : « on fera plutôt comme
  Neo4j pour l'instant » ; pas sur la stèle, chantier du moteur à cadrer quand une marche
  le demande.
- **Gravité** : réponse incomplète possible après un arrêt brutal (l'index en avance ou en
  retard sur la base), dite par l'état d'index, jamais muette ; pas de perte de données.
- **Atteignable en service** : oui, c'est le défaut (`RAG3WEAVER_FTS=fichiers` depuis
  `21a1d67c5`).
- **Touche rag3weaver** : oui (l'état d'index « mots : en cours / à rebâtir », les gardes
  de réouverture, le banc d'arrêt brutal).

## Ce que c'est

Depuis les nouveaux défauts, l'index plein texte (lucivy) vit dans des fichiers
`<base>.fts/` à côté du fichier de la base, et non plus dans des lignes blob du moteur.
Il n'est donc plus dans la même transaction que les données : une annulation, un point
de reprise ou un rejeu après une mort couvrent la base, pas l'index. La cohérence est
réparée après coup par rag3weaver (état d'index, rebâti), pas garantie par construction.

## Pourquoi on ne revient pas aux blobs

Le mode blobs donnait l'atomicité (un seul journal pour tout), mais à un coût qui n'était
pas un réglage : des segments qui se réécrivent sans cesse à travers un journal à
25 octets par cellule (`2026-10-05-cout-par-cellule-au-journal.md`), un point de reprise
forcé à chaque COPY, et un moteur qui ne récupère pas l'espace de ses lignes supprimées
(purge côté rag3weaver `d1aa7d296` en attendant). Sur disque, la poussée des blobs était
le premier poste du premier index (346 s sur 823 s par paquets de 512). Il faudrait au
moteur deux chantiers qu'il n'a pas, la récupération d'espace et un chemin pour les gros
objets hors du journal ligne à ligne, avant que les blobs redeviennent bons. Le mode blobs
reste une option testée (`RAG3WEAVER_FTS=blobs`, série de confirmation du 10 octobre :
89 → 87 s) jusqu'à une vraie publication, où l'on tranchera de le garder ou de le retirer.

## Ce que font les moteurs établis

Neo4j garde ses index plein texte (Lucene) dans des fichiers hors du magasin, et c'est le
moteur qui les tient cohérents au point de reprise : instantané de l'index pris avec celui
du magasin, rejeu des deux ensemble à la reprise. PostgreSQL garde chaque index dans son
propre fichier, sous le même journal que la table. Dans les deux cas l'utilisateur voit une
base qui tient ou s'annule d'un bloc, sans qu'un seul journal porte tout.

## Recette

Pas de recette Cypher : le défaut est entre deux systèmes. Témoin de la forme actuelle :
le cas « tués » d'`e2e_arret_brutal` (rag3weaver), qui montre l'état d'index après une
mort pendant une indexation, et la réparation.

## Ce qu'il faut pour le fermer

1. Le point de reprise de rag3db connaît les fichiers d'index déclarés par l'extension
   (ou par rag3weaver) : il prend leur instantané dans le même geste que le magasin, et le
   rejeu les ramène au même point (comme Neo4j).
2. Une annulation de transaction rend l'index à l'état d'avant (lucivy sait commiter par
   shard ; il faut un point d'annulation aligné sur celui du moteur, ou le rebâti borné du
   delta).
3. Le banc d'arrêt brutal joue « mort entre le point de reprise du magasin et celui de
   l'index » et prouve que la base rouvre cohérente sans rebâti.
4. L'état d'index reste, pour dire ce qui se passe pendant un rebâti volontaire.

Ni l'un ni l'autre n'est commencé. À cadrer avec le cœur C++ (le point de reprise) et
l'arbre principal (l'état d'index), page courte d'abord.
