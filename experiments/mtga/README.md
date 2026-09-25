# MTGA — API locale et source d’ingestion structurée

Prototype dans `experiments/mtga/`, entièrement ignoré par Git. FastAPI sert un snapshot SQLite en lecture seule.
Aucun scraping : collection via `mtga-reader`, decks via `Player.log`, textes et glossaire depuis les bases locales d’Arena.
Arena peut être fermé après la capture. L’adaptateur rag3weaver et le constructeur de decks restent à implémenter.

## Démarrer

Depuis la racine du dépôt :

```bash
bash experiments/mtga/scripts/serve.sh
```

API : http://127.0.0.1:8731 — documentation : http://127.0.0.1:8731/docs — OpenAPI : http://127.0.0.1:8731/openapi.json

## Découvrir la structure et ingérer

Commencer par `GET /v1/schema`. La réponse décrit :

- Les datasets, leur disponibilité, leur nombre d’entrées et leurs relations.
- Chaque champ de payload : `text`, `keyword`, `integer`, `boolean`, `enum`, `datetime`, `reference`, `array`, `object`.
- Les éléments des listes, champs imbriqués, valeurs des enums, nullabilité et champs obligatoires.
- Le JSON Schema complet, issu des mêmes modèles Pydantic qui valident les records.
- Des suggestions de types d’index. Aucun index Qdrant ni embedding n’est créé.

| Dataset | Record | Contenu |
| --- | --- | --- |
| `cards` | `card` | Catalogue, textes FR/EN, tableau natif `abilities`, références aux faces et mécaniques |
| `collection` | `collection_entry` | Référence `card_id`, quantité entière, profil et date de capture |
| `decks` | `deck` | Nom, format sauvegardé, entrées structurées : référence carte, quantité, emplacement |
| `mechanics` | `mechanic` | Glossaire, définitions FR/EN, provenance et disponibilité de la définition |
| `boosters` | `booster_holding` | Schéma réservé ; inventaire absent de la capture, `available=false`, nombre inconnu |

Un record possède `id` (UUIDv5), `logical_id`, `schema_version`, `snapshot_id`, `kind`, `source_uri`, `payload` et `text`.
**`payload` est la donnée structurée ; `text` est seulement une projection pour l’indexation textuelle.**
Les quantités ne sont pas enfouies dans du texte. La force `*` ou `1+*` reste une chaîne ; `power_numeric` vaut null si elle n’est pas un entier fixe.
Les boosters désignent un lot d’un produit, pas des boosters individuels dont l’identité serait inconnue.

| Route GET | Usage |
| --- | --- |
| `/v1/status` | Snapshot, dates, compteurs, jokers et diagnostics |
| `/v1/schema` | Catalogue des payloads typés et des datasets |
| `/v1/records?dataset=collection` | Records typés, paginés |
| `/v1/records/{uuid}` | Résoudre une identité ou suivre une référence |
| `/v1/record-exports/collection.jsonl` | Dataset entier, un record structuré par ligne |
| `/v1/collection?q=vol` | Vue enrichie des cartes possédées et de leurs textes |
| `/v1/cards?q=lifelink&owned_only=true` | Recherche par mots/préfixes dans les noms, types et capacités |
| `/v1/cards/{arena_id}` | Carte et faces associées, tableau natif des capacités |
| `/v1/decks?name=vampire` | Recherche de decks par fragment de nom |
| `/v1/decks/{arena_deck_id}` | Deck avec toutes ses cartes enrichies |
| `/v1/mechanics?q=defense%20talismanique` | Recherche de mécaniques en FR/EN, sans distinction de casse/accents |
| `/v1/mechanics/Flying` | Définition d’une entrée par sa clé source exacte |
| `/v1/quality` | Diagnostic des alias, définitions équivalentes et règles de rapprochement |

`dataset` accepte `cards`, `collection`, `decks`, `mechanics`, `boosters`. Un dataset indisponible renvoie 409, pas une fausse liste vide.
Les listes acceptent `limit` (1–500) et `offset`. Réponse : `snapshot_id`, `total`, `limit`, `offset`, `next_offset`, `items`.
`next_offset=null` indique la fin. Ordre stable par identifiant natif/clé.

Lire `/v1/status`, puis passer `snapshot_id=...` sur toutes les pages/datasets d’une ingestion.
Si les données changent entre deux requêtes : HTTP 409, recommencer. Inconnue : 404 ; paramètre invalide : 422 ; base absente : 503.

```bash
curl -s http://127.0.0.1:8731/v1/schema
curl -s 'http://127.0.0.1:8731/v1/records?dataset=collection&limit=2'
curl -s 'http://127.0.0.1:8731/v1/decks?name=vampire'
curl -s http://127.0.0.1:8731/v1/cards/93771
curl -s http://127.0.0.1:8731/v1/mechanics/Flying
```

Client HTTP d’ingestion fonctionnel :

```bash
python3 experiments/mtga/scripts/fetch_source.py --dataset collection --output /tmp/mtga-collection-records.jsonl
python3 experiments/mtga/scripts/fetch_source.py --dataset decks --output /tmp/mtga-deck-records.jsonl
```

Pour rag3weaver, ingérer `cards`, `collection`, `decks`, `mechanics` et conserver leurs relations par UUID.
Les enregistrements de collection se prêtent surtout aux filtres et jointures, pas à un embedding.
Les anciennes routes `/v1/documents` et `/v1/exports/{dataset}.jsonl` restent des projections texte/métadonnées ; utiliser `/v1/records` et `/v1/record-exports` pour le contrat typé.

## Identité, doublons et capacités

Les UUID sont déterministes à partir d’une clé logique et d’un namespace fixe dans `identity.py` :

- Carte : `mtga:card:{arena_id}`, identité par impression/face. Les réimpressions et versions rééquilibrées restent distinctes.
- Collection : `mtga:{source_id}:collection:{arena_id}`. Une quantité modifiée ne change pas l’identité.
- Deck : `mtga:{source_id}:deck:{arena_deck_id}`. Renommer le deck ne change pas son identité.
- Mécanique : `mtga:mechanic:{client_key}`. Les clés distinctes restent traçables même si leurs définitions se ressemblent.

Conserver `data/source.json` : il identifie ce profil local et n’est pas un identifiant de compte. Ne pas mélanger plusieurs comptes dans ce dossier.
Une réingestion fait des upserts par `id`, puis retire de l’index les records qui ont disparu du dataset.

**`abilities` vient directement de `Cards.AbilityIds`, joint à la table `Abilities` et aux localisations.**
Chaque élément contient `ability_id`, `text_id`, les codes natifs, `text_en`, `text_fr` et les références exactes au glossaire.
Une capacité peut être une phrase entière ; elle n’est pas arbitrairement découpée en effets.
`ability_id` identifie la capacité native ; le couple `(ability_id, text_id)` permet de conserver sa présentation exacte.

`mechanics` contient les égalités de libellé entier ou de titre balisé avec le glossaire.
`mechanic_mentions` contient séparément les mentions lexicales (limites de mots, expressions longues prioritaires).
Une carte qui parle de créatures avec le vol n’est donc pas déclarée comme possédant elle-même le vol.
Les conditions et capacités octroyées ne sont pas évaluées : ce prototype n’est pas un moteur de règles.

Normalisation de diagnostic : Unicode NFKC, casse et espaces. Pas de fusion sur simple inclusion de sous-chaîne.
Les accents sont ignorés uniquement pour la recherche du glossaire. `/v1/quality` expose les groupes de libellés/définitions équivalents sans effacer les clés sources.

Le glossaire provient des infobulles `AbilityHanger/Keyword/*` du client. Ce sont des explications pratiques, pas les règles complètes.
Une définition absente est signalée (`definition_status=missing`) ; les paramètres non résolus aussi (`has_placeholders`).
Les règles complètes restent consultables sur https://magic.wizards.com/en/rules ; elles ne sont pas copiées dans ce prototype.
L’association au glossaire est volontairement partielle, et n’assimile pas `Descend 4` à l’action générique `Descend`.

## Rafraîchir

Arena ouvert et connecté à l’accueil, journaux détaillés activés :

```bash
bash experiments/mtga/scripts/refresh.sh
```

La lecture mémoire doit être exécutée hors du bac à sable si celui-ci masque Arena.
Sur cette machine, elle a réussi sans `sudo` hors du bac à sable.
Elle capture seulement la collection, sans lire les jetons du compte. Un échec conserve l’ancien export.
Ne pas lancer plusieurs rafraîchissements simultanés.

Pour reconstruire le snapshot depuis la capture existante :

```bash
python3 experiments/mtga/scripts/build.py
```

Le script prend la base Arena la plus récemment modifiée. `--raw-dir` et `--log` permettent de modifier les chemins Steam/Proton.
La base servie est remplacée atomiquement. L’API voit le nouveau snapshot à la requête suivante.
Les decks proviennent du dernier `StartHook` : redémarrer Arena avant une capture définitive si des decks ont été modifiés.
Les formats sauvegardés ne garantissent pas la légalité actuelle. La fusion des impressions, les terrains de base,
les règles de construction et la génération de decks nécessitent une couche métier supplémentaire.

## Fichiers et vérification

`data/arena.sqlite` est la base servie ; `collection.json` la capture brute ; `source.json` l’identité persistante.
`cards.jsonl`, `owned-cards.jsonl`, `decks.json`, `mechanics.json` sont des exports locaux structurés.
Le fichier historique `documents.jsonl` est une projection simple ; préférer les routes de records pour l’ingestion.

```bash
cd experiments/mtga
npm ci --ignore-scripts --no-audit --no-fund
uv venv .venv
uv pip install --python .venv/bin/python -r requirements.txt
.venv/bin/python -m unittest -v test_api
```

Les tests valident tous les payloads du snapshot, l’unicité et les références des UUID,
les capacités natives, le glossaire, la pagination, les exports et les erreurs.
