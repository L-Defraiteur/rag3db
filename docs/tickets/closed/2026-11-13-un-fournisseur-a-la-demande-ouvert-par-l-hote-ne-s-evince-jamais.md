# Un fournisseur à la demande ouvert par l'hôte ne s'évince jamais

- **État** : fermé (14 novembre 2026, embarquement) : l'embarqueur activé à la demande est sur master
  (5d897d094), et le vrai jeu par l'hôte de luciepc est vert (binaire 012be4a58, lib 0c41f9fe6). Ouvert
  le 13 novembre, session embarquement, à la demande de c7.
- **Famille** : produit
- **Gravité** : coût — la VRAM d'un modèle « à la demande » reste prise tant que le backend qui l'a
  ouvert vit, et un backend ouvert charge un modèle dont aucun index n'aura besoin
- **Atteignable en service** : oui, dès qu'un manifeste déclare une activation
  `Residence::ADemande` (aucun manifeste de Lucie ne le fait aujourd'hui)
- **Touche rag3weaver** : oui (`src/backend.rs`, `src/model_source/activation.rs`)

## Ce que c'est

`Backend::embedder_or_absent` appelle `model_source::connect_embedder` à l'ouverture du backend.
Pour une source qui déclare une activation, `connect_embedder` lance le fournisseur (local ou
distant) et prend un bail d'usage (`DaemonEmbedder::prendre_bail`, master `7042128d0`), renouvelé
par un fil tant que le client vit. L'hôte garde ce client tant que le backend est ouvert. Donc :

1. **l'éviction ne joue pas côté hôte** : le bail ne tombe jamais, le fournisseur déclaré
   `a_demande` (master `0ee027cf0`) reste chargé aussi longtemps que le backend ;
2. **un backend ouvert lance le modèle sans index** : l'ouverture seule paie le chargement et la
   VRAM, même si aucune écriture ni recherche n'a besoin de vecteurs.

Lu dans le code le 13 novembre (master `f3897600f`) ; pas encore de témoin.

## Ce qu'il faut pour le fermer

Un embarqueur activé à la demande pour `Residence::ADemande` : rien à l'ouverture ; activation au
premier calcul, bail tenu tant que les appels se suivent et rendu après un délai nommé, plus court
que l'inactivité du fournisseur ; réactivation transparente (une fois, identité revérifiée) si le
fournisseur est parti. Témoins rouges d'abord sur la fixture CPU : ouvrir sans lancer ; un index
qui lance et a ses vecteurs en moins d'une minute ; l'éviction après l'index et le délai ; un second
index qui relance ; une lecture sans besoin de vecteurs qui n'active rien, et une qui en a besoin
qui active. `Residence::Maintenir` et les services posés à la main ne changent pas.

## Le lot (13 novembre au soir, embarquement)

`model_source::activation::connecter_ou_attendre` : une déclaration `ADemande` rend un
`ALaDemande`, qui dit le modèle et la dimension déclarés sans rien lancer ; son premier `embed`
active (par `connecter`, local ou distant, identité vérifiée) ; un fil rend le client (donc le
bail et, à distance, le tunnel) après `rendre_le_bail_secs` sans appel ; un `embed` qui trouve le
fournisseur parti (« ne répond pas », délai) réactive une fois. Les deux délais sont nommés dans les
règles (`docs/embarquement/13-11-2026/01-…`, « À la demande : les deux délais »). La fixture compte
les baux et s'évince comme le service réel. Témoins : `a_la_demande_*` (lib) et
`e2e_embarqueur_a_la_demande` (une lecture par les mots n'active rien, une lecture dense active).


## Le vrai jeu par l'hôte de luciepc (14 novembre 2026, 13 h 51 → 13 h 55, embarquement)

Le jeu passe par **l'hôte de luciepc** (7970, binaire posé 012be4a58, lib 0c41f9fe6), sur un **index
de mesure à part** :
- **Le manifeste** : `~/.cache/rag3weaver-build/vrai-jeu/backend.json` à luciepc, construit par
  `manifeste.py` depuis le gabarit de code de 012be4a58.
- **La base et le dépôt** : une base neuve, et une copie de `codeparsers/src` (79 fichiers, 2194
  morceaux).
- **La déclaration** : `models.embed` = granite-107m, 384 dimensions, adresse `127.0.0.1:7890`, et
  une `activation` locale à luciepc :
  - le programme est le binaire du service bâti sur 012be4a58, empreinte `781b48f5ffa8eeef` ;
  - `RAG3WEAVER_BURN_DEVICE_EMBEDDER=gpu:0` (card2), `peripherique: card2` ;
  - `residence: a_demande { inactivite_secs: 60, rendre_le_bail_secs: 30 }`.
- **Sans creux** : le manifeste n'a pas de `models.sparse`, pour que l'index de mesure ne touche pas
  le bge-m3 partagé.
- **Hors périmètre** : aucun manifeste de Lucie touché ; card0 jamais ouverte ; les deux services
  résidents de card2 (7878, 7879) restent seuls à côté.
- **La place** : `poste lourd` tenue pendant toute la séquence.

| Pas | Mesuré |
|---|---|
| Ouverture par l'hôte (`/backend/open`, puis `describe`) | aucun processus du fournisseur, 7890 ne répond pas ; card2 20 395 Mio, card0 88 Mio, inchangées |
| Index 1 (`index`, 79 fichiers) | le fournisseur est lancé pendant l'appel ; identité vérifiée au `/sante` : granite-107m, 384 dim., `usage { baux: 1 }`, `eviction { inactivite_s: 60 }`, `activation { profil: vrai-jeu-a-la-demande, peripherique: card2, recette 217f7450… }` ; index et vecteurs (2194) en 9,2 s |
| Bail rendu | `baux: 0` à **+29,5 s** après la fin de l'index (délai déclaré : 30 s) |
| Éviction | plus de processus à **+60,5 s** après le bail rendu (délai déclaré : 60 s) ; card2 revenue à 20 395 Mio |
| Index 2 (un fichier ajouté, 80 fichiers) | **réactivation** : nouveau pid 4,0 s après l'appel, même identité (même recette), vecteurs prêts (2196) en 6,2 s |
| Bail rendu, éviction | `baux: 0` à +30,5 s, plus de processus à +59,6 s ; card2 à 20 401 Mio |
| Fermeture (`/backend/close`) | `closed: true` ; aucune unité `rag3weaver-fournisseur-*`, aucun processus |

**La mémoire, carte par carte**, échantillonnée toutes les 2 s (87 relevés) :
- **card2** : de 20 395 Mio (les deux services résidents) à **26 087 Mio au plus fort** ; le
  fournisseur tenait **5,7 Gio au plus fort** pendant ses vecteurs (2,8 à 3,9 Gio aux autres
  relevés, `drm-total-vram` de son processus). Après chaque éviction, card2 revient à sa valeur
  de départ.
- **card0** : **88 Mio à chaque relevé**, jamais ouverte.

**Un premier essai, cassé par mon script (13 h 49)** : il prenait la réponse de `wait_output`, qui
recopie le motif cherché, pour la fin de l'index, puis plantait sur une ligne morte. Le backend est
resté ouvert, l'index en cours ; ce premier index s'est fini seul, son bail est tombé, puis
l'éviction a suivi à 60 s, card2 revenue à 20 395 Mio. La mesure ci-dessus est rejouée depuis une
base neuve, la fin de l'index étant lue **au journal** (sa dernière ligne).

Scripts et journaux à luciepc : `~/.cache/rag3weaver-build/vrai-jeu/` (`mesure.py`, `sonde.sh`,
`mesure.out`, `sondes.log`, `mesure-1-casse.out`).
