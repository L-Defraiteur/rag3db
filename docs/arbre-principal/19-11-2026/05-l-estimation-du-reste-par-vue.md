# L'estimation du reste à indexer, par vue

19 novembre 2026, arbre principal, à la demande de c7. C'est ma part du point 7 de la vision du 15 novembre (« la
recherche attend l'index si c'est court »). `search.index_wait_s` côté socle est à archi, et la réponse des outils à
recherche. Ticket : `docs/arbre-principal/tickets/2026-11-16-l-estimation-du-reste-d-une-vue-par-phase.md`.

## Ce que la vue expose

`views::view_state` gagne deux champs, en secondes, `None` quand il manque un terme :

- **`text_eta_s`** : les fichiers dus de la couche de la vue (`Lag`, la couche de la vue) multipliés par le débit
  observé de réindexation (`Lag::rate`, la moyenne glissante `index_rate_ms_per_file`, gardée au catalogue). Ce n'est
  jamais une constante.
- **`vectors_eta_s`** : les morceaux dus de la vue, sans vecteur, multipliés par leur taille moyenne et divisés par le
  débit d'embarquement du service de la vue.

## Le débit d'embarquement, et d'où il vient

Dans cet ordre :

1. **La clé du catalogue**, `embedding_rate:{modèle}:{adresse}@{poste}` (embarquement, bef0b27cb3), lue par
   `known_embedding_rate`. C'est un débit d'index : réclamation, appel, écriture et reconstruction compris. C'est
   donc le bon débit pour un reste à attendre.
2. **Sinon, le débit déclaré du banc**, multiplié par un facteur dit comme tel. Le banc ne mesure que les appels au
   service, et sur B′ le débit d'index valait 47 % du banc (92 152 contre 194 422 car./s). Le facteur de départ est
   **0,5**. La réponse le dit : « d'après le banc × 0,5 ». La forme de la déclaration est à embarquement ; la
   proposition du banc est un fichier par poste, `~/.config/rag3weaver/embedding-rates.json`, sous la même clé sans
   préfixe.
3. **Sinon, `None`** : « débit inconnu ». Jamais une valeur inventée.

## Les témoins, rouges d'abord

1. Une vue avec N fichiers dus et un débit de réindexation de r ms par fichier rend `text_eta_s` ≈ N × r / 1 000. Le
   débit change, l'estimation suit.
2. Une vue avec M morceaux dus de taille moyenne c, sous la clé de débit d de son service, rend `vectors_eta_s` ≈
   M × c / d. Sans la clé et avec un débit déclaré, elle vaut M × c / (0,5 × d_banc), et elle le dit. Sans l'un ni
   l'autre, elle vaut `None`.
3. Une autre vue, sans rien de dû, rend 0 et 0 pendant ce temps.
