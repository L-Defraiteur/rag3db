# Lot 4 — un contrôleur est un graphe, une route y mène, une vue le rend

11 octobre 2026, session « everything declarative » (chantier I). Un backend
déclaré sert des pages : une adresse mène à un outil déclaré, son résultat va
à un gabarit HTML. Trois façons d'entrer dans le même graphe : l'outil
(l'agent), l'événement (une réaction), la route.

## 1. Le dossier `nodes/` dans un backend (4a)

- `PreparedBackend::load` découvre `nodes/` à côté du manifeste (la forme du
  lot 2 : `<nom>.node.json` + son script) et enregistre chaque nœud déclaré
  sous son nom ; un rechargement relit le dossier.
- Refusés au chargement, en nommant le fichier : une déclaration fautive, un
  nom qui est celui d'un nœud fourni.
- Un nœud scripté est **pur** (ni fichiers, ni réseau, ni commande) : il entre
  dans toutes les politiques où entre un calcul — outils, crochets après
  outil, crochets de validation, réactions —, au chargement comme à
  l'exécution, sans toucher aux listes de sécurité de `backend_code.rs`.
- Un paramètre de configuration d'un nœud scripté prend le type de son schéma
  (`string`, `integer`, `number`, `boolean`, sinon JSON) : un outil qui lie
  `name string!` à `{"type": "string"}` est vérifié comme pour un nœud
  fourni.

## 2. Les routes et les vues (4b)

```json
"routes": {
  "GET /hello/{name}": { "tool": "greet", "view": "hello.html" },
  "GET /echo":         { "tool": "echo" }
}
```

- **Le contrôleur est un outil déclaré** : mêmes paramètres, mêmes
  validateurs, même politique, même version des déclarations que quand
  l'agent l'appelle. Le contrat de données n'est pas réécrit.
- La requête devient les arguments : corps (un objet JSON, ou un formulaire),
  puis requête, puis chemin ; une même clé donnée deux fois est refusée en la
  nommant. Une valeur de chemin ou de requête est du texte, lue comme JSON
  quand le paramètre de l'outil n'est pas une chaîne.
- **La vue** est un gabarit du dossier `views/` (découvert, comme `nodes/` ;
  `extends` et `include` entre vues) ; elle reçoit `result`, `arguments` et
  `response`. Les vues `.html`, `.htm`, `.xml` échappent ce qu'elles
  reçoivent. Sans vue, la route rend le résultat en JSON.
- **La page et le résultat sortent du même appel** : `Backend::route` rend
  les deux (`RouteResponse { status, content_type, body, result }`).
- Refusés au chargement — donc au rechargement, l'ancienne version restant en
  service — : une clé mal formée, une méthode inconnue, une route vers un
  outil non déclaré, une vue absente ou qui ne se compile pas (avec sa
  ligne), deux routes ambiguës. À la requête : 404 (aucune route), 405 (la
  route existe pour une autre méthode, qui est dite), 422 (l'outil refuse,
  avec sa raison).
- Les littéraux passent devant les paramètres (`/products/new` avant
  `/products/{key}`).

## 3. `serve` (4c)

```text
rag3weaver-backend serve --manifest <backend.json> [--address 127.0.0.1:7777] [--threads 4]
```

- `src/serve.rs`, sur `tiny_http` (feature `daemon`, déjà au crate) : chaque
  requête va à `Backend::route`. Plusieurs fils ; le backend est partagé, un
  appel prend sa version au départ.
- **Boucle locale seulement** : le proto n'a pas d'authentification ; une
  adresse hors de la boucle est refusée. Les gardes fournies viendront avec
  la vision §9.
- **`POST /arret`** ferme l'écoute et rend la main : le binaire ferme alors la
  base proprement (`shutdown`) — jamais un `exit` qui couperait le journal.
- Le décodage (`%xx`, `+`, formulaire, JSON) est une fonction pure, éprouvée
  sans réseau.

## 4. Les témoins

Rouges d'abord (4a : 4 ; 4b : 7 routes avec un bouchon, puis 8 avec le
typage ; 4c : à la compilation), puis verts. `src/backend_reload_tests.rs`,
sur le backend jouet monté sans base, et `src/serve.rs` :

- un nœud de `nodes/` sert un outil, est rechargé, est refusé sous un nom
  fourni ou s'il est fautif ;
- une route rend sa vue, échappe ce qu'elle reçoit, rend du JSON sans vue ;
  404 et 405 ; route vers un outil inconnu et vue absente ou cassée refusées
  au chargement ; une vue modifiée change la page après rechargement ;
- `serve` répond sur HTTP (un vrai port éphémère : 200 et `text/html`, 404),
  s'arrête par `POST /arret` et rend le backend à son appelant seul ;
  refuse une adresse hors de la boucle ; le décodage des requêtes.

## 5. La suite

- **4d — le backend jouet** (produit, catégorie, commande), avec une liste,
  une fiche et un formulaire, et son e2e sur une vraie base : la recette du
  §1 de la page du proto commence là.
- La montre sur les fichiers est dans l'hôte, écrite par la session mémoire
  avec `watch_bound` ; elle appellera `check_reload` / `apply_reload`.
