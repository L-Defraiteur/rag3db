# Le banc d'étalonnage du débit d'embarquement

- **État** : ouvert ; la page de conception d'abord, soumise à c7 et à embarquement, et pas de code
  avant leur accord.
- **Famille** : produit
- **Gravité** : moyenne. Le seuil de 6 s de Lucie (`search.index_wait_s`) se dit en secondes à
  l'utilisateur et en lots au moteur. Sans débit mesuré par service, l'hôte ne sait pas convertir.
- **Atteignable en service** : oui, à chaque recherche pendant un index.
- **Touche rag3weaver** : oui, à terme : l'écriture du débit au catalogue.
- **Pour** : une session Codex (passation du 16 novembre). La table par service est à embarquement.
- **Signalé par** : Lucie, 15 novembre, 21 h (« traduire par un bench d'avance à quoi correspondent
  6 secondes vs combien de lots à embarquer »).
- **Ouvert le** : 16 novembre 2026

## Ce qui existe

- `Rate` (chars_per_second) au catalogue. Il est noté par `note_measured_embedding_rate` à la fin
  d'un index de 512 morceaux et plus, et relu par `known_embedding_rate` pour le « reste environ
  N min ». Il est par catalogue, pas par service ni par poste.
- 14a78135f1 : un outil de lecture attend sous `search.index_wait_s`, ou répond « index pas prêt,
  ~N s », à partir du débit observé par fichier, gardé d'une ouverture à l'autre.
- Les lignes `[montre]` des journaux d'index, par exemple « 56 morceaux embarqués hors du verrou :
  16 474 ms ». Un débit en morceaux par seconde, par passe et par service, s'en tire après coup.
- La décision de l'orchestration (`docs/orchestration/14-11-2026/03-la-file-en-pile.md`, 21 h 07) :
  le banc écrit au même endroit que l'index, une seule source pour l'hôte. Le banc dessine le jeu
  et la mesure, embarquement la table par service.

## Ce qu'il faut faire

1. **La page de conception**, `docs/banc/<date>/NN-le-banc-d-etalonnage.md` :
   - le jeu : des lots fixes de 1 500 caractères, la taille du découpeur, tirés de vrais
     `ScopeContent_Chunk`. On peut lire leurs textes dans une copie de B′
     (`banc-parite/migration-bprime-20261116`). Le jeu est figé dans un fichier, avec son
     empreinte ;
   - la mesure : par service (7878 granite-278m, 7879 bge-m3, 7880 granite-107m), par poste
     (portable et luciepc), en GPU et en CPU si le service le permet. Lots de 1, 8, 32 et 64, trois
     répétitions, et la première passe de chauffe à part. Sortie : lots par seconde, caractères par
     seconde, p50 et p90 ;
   - l'écriture : la forme de la table par service (adresse, modèle, poste) à convenir avec
     embarquement. Le banc ne touche pas au catalogue avant cet accord ;
   - la vérification : le débit du banc comparé au débit des lignes `[montre]` d'un index réel du
     même service.
2. Soumettre la page à c7 et à embarquement, et attendre leur accord.
3. Puis la mesure, en client des services : jamais un service ajouté sur card2, jamais card0.

## La preuve

- La mesure commence par prouver le service mesuré : la réponse de `/sante`, le modèle et la
  dimension, avant tout chiffre.
- L'écart entre le débit du banc et celui d'un index réel est dit, avec ses causes.

## Taille

La page, deux heures. La mesure, quatre heures après l'accord.
