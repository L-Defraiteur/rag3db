# Embarquement : la déclaration du débit d'embarquement par poste

Demande de c7, le 19 novembre au soir. C'est la pièce d'embarquement du point 7 de la vision du 15 novembre (« la
recherche attend l'index si c'est court ») ; la lecture côté hôte est à arbre
(`docs/arbre-principal/19-11-2026/05-l-estimation-du-reste-par-vue.md`).

## Le manque

Une base neuve n'a pas encore de débit au catalogue : la clé `embedding_rate:{modèle}:{adresse}@{poste}` (lot
bef0b27cb3) ne s'écrit qu'à la fin d'un index de 512 morceaux et plus. Pendant ce temps, l'hôte ne peut pas estimer
le reste à attendre. Le banc, lui, a mesuré le débit de chaque service (`docs/banc/18-11-2026/etalonnage-luciepc.json` :
p50 de 194 422 car./s au 7878, 42 142 au 7879), sous la même clé.

## La forme

**Un fichier par poste**, déclaré, que l'hôte lit et n'écrit jamais :

- **Chemin** : `$XDG_CONFIG_HOME/rag3weaver/embedding-rates.json`, sinon `~/.config/rag3weaver/embedding-rates.json`.
  `RAG3WEAVER_EMBEDDING_RATES` donne un autre chemin, absolu ; vide, aucune déclaration n'est lue.
- **Contenu** : les clés sont celles du catalogue sans le préfixe `embedding_rate:`, c'est-à-dire
  `{modèle}:{adresse}@{poste}`, ou `{modèle}:local@{poste}` pour un modèle en processus :

```json
{
  "version": 1,
  "rates": {
    "granite-278m:127.0.0.1:7878@LuciePC": {
      "chars_per_second": 194422,
      "measured_at": "2026-11-18",
      "source": "banc, docs/banc/18-11-2026/etalonnage-luciepc.json, p50"
    },
    "bge-m3:127.0.0.1:7879@LuciePC": {
      "chars_per_second": 42142,
      "measured_at": "2026-11-18",
      "source": "banc, docs/banc/18-11-2026/etalonnage-luciepc.json, p50"
    }
  }
}
```

- **Qui l'écrit** : le banc, depuis son étalonnage. Rien ne l'écrit dans le dépôt : c'est un fait du poste.

## La lecture

`Catalog::embedding_rate_for_wait()` rend le débit et sa source, dans cet ordre :

1. **`index`** : la clé du catalogue (puis l'ancienne `{modèle}:service|local`, comme `known_embedding_rate`). C'est
   un débit d'index : réclamation, appel, écriture compris.
2. **`bench`** : la déclaration, sous la clé de l'embarqueur (`{modèle}:{adresse prise}@{poste}`), multipliée par
   **0,5** (`BENCH_RATE_FACTOR`). Le banc ne mesure que les appels au service ; sur B′, le débit d'index valait 47 % du
   banc. La source porte la date de la mesure.
3. **`unknown`** : ni l'un ni l'autre. Jamais une valeur inventée.

**Une déclaration illisible** (JSON faux, version inconnue, débit non positif) ne fait pas échouer l'hôte : la source
est `unknown`, et la raison est dite (« déclaration illisible : … »).

**Dans l'état** (à convenir avec arbre) : `vectors_rate_source`, qui vaut `"index"`, `"bench"` ou `"unknown"`. À
l'affichage : « débit de l'index », « débit du banc × 0,5 (mesuré le 2026-11-18) », « débit inconnu ».

## Les témoins, rouges d'abord

`src/catalog/progress.rs`, tests :
1. **Une base sans clé, une déclaration pour son service** : le débit vaut 0,5 × le déclaré, et la source est
   `bench`, avec sa date.
2. **La clé du catalogue l'emporte** sur la déclaration (source `index`).
3. **Sans déclaration, ou une déclaration pour une autre adresse ou un autre poste** : `None`, source `unknown`.
4. **Une déclaration illisible** : `None`, source `unknown`, et la raison est dite.

Les témoins donnent le fichier par un chemin explicite (`embedding_rate_for_wait_from(Some(chemin))`), dans un
dossier temporaire : jamais le vrai `~/.config`, et aucune variable d'environnement posée par un test parallèle.
`embedding_rate_for_wait()` résout le chemin (variable, puis XDG), puis l'appelle.

## Taille

Deux heures, témoins compris.
