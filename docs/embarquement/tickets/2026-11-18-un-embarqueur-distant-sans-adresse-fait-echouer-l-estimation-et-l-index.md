# Un embarqueur distant sans adresse fait échouer l'estimation et l'index

- **État** : ouvert (18 novembre 2026, embarquement ; relevé en évaluant la clé du débit de Codex)
- **Pour** : embarquement
- **Famille** : produit
- **Gravité** : haute pour qui l'emploie. Un backend dont `models.embed` est en `provider: compatible` ne peut ni
  estimer ni finir un index de 512 morceaux et plus. Lu dans le code, pas encore joué.
- **Atteignable en service** : oui, pour tout manifeste dont l'embarquement passe par un fournisseur `compatible`
  (`HttpEmbedder`).
- **Touche rag3weaver** : oui (`src/http_embedder.rs`, `src/catalog/progress.rs`, `src/dataflow/index_nodes.rs`)

## Ce que c'est

Depuis la clé du débit par adresse et poste (master bef0b27cb3), `Catalog::note_measured_embedding_rate` **refuse**
un embarqueur distant qui ne déclare pas d'adresse (`called_address`) : « embedding rate provenance is unknown; no
rate was recorded » (`embedding_rate_place`, `src/catalog/progress.rs`). Le refus est voulu, pour qu'une provenance
inconnue ne se note jamais comme un calcul local.

Mais :
- `HttpEmbedder` (`src/http_embedder.rs`), que `model_source.rs:559` construit pour `models.embed` en
  `provider: compatible`, se dit distant (`distant()` vrai), sans `called_address` ;
- `run_index` appelle `note_measured_embedding_rate(…)?` à la fin d'un index de 512 morceaux et plus
  (`src/dataflow/index_nodes.rs:675`), et l'estimation appelle `probe_embedding_rate(…)` sous « sonde : … »
  (`index_nodes.rs:169`), qui passe par la même écriture.

Le refus remonte donc par `?` : l'estimation d'un premier index échoue, et un index de 512 morceaux et plus finit en
échec alors que ses vecteurs sont écrits.

## Ce qu'il faut faire

1. `HttpEmbedder::called_address` rend son adresse (celle qu'il appelle, comme `DaemonEmbedder`).
2. Un débit qui ne peut pas se noter ne fait jamais échouer un index ni une estimation : `run_index` et la sonde le
   disent au journal (« débit non noté : provenance inconnue ») et continuent. Le refus de
   `note_measured_embedding_rate` reste pour un appel direct.

**Ce qui ne se touche pas** : la clé (`embedding_rate:{model}:{address}@{host}`) et son repli sur l'ancienne.

## Comment le prouver

- `an_unknown_provenance_never_fails_the_index` (lib, `index_nodes.rs`) : un faux embarqueur distant sans adresse ;
  un index de 512 morceaux et plus finit par « indexation terminée », et le journal dit que le débit n'est pas noté.
  Rouge d'abord.
- `the_http_embedder_declares_the_address_it_calls` (lib, `http_embedder.rs`).
- Suites exigées par chemins : `e2e_code_sync` (`catalog*`), `e2e_lectures` (le journal d'index).

## Taille

2 à 3 heures.
