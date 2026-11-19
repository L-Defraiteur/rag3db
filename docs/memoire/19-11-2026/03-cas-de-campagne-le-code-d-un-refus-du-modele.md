# Cas de campagne : le code d'un refus du modèle n'est gardé nulle part

```json
{
  "parent": "bcdac268f5e73a6fb3a1ed603d6abcee7d997f2b",
  "tree": "extension/rag3weaver",
  "paths": [
    "extension/rag3weaver",
    "tools/rust_api"
  ],
  "submodules": {
    "extension/rag3weaver/codeparsers": "72ab7a80ea4f9a371ec0e7a74288f4a9e312547f"
  },
  "needs_lib": false,
  "witness": "agent::tests::llm_call_event_keeps_each_retry_reason_and_wait",
  "test": "cargo test -j 8 --lib -- --exact agent::tests::llm_call_event_keeps_each_retry_reason_and_wait",
  "suites": [
    "cargo test -j 8 --lib"
  ],
  "hints": [
    "extension/rag3weaver/src/agent.rs"
  ],
  "ours": "2d5b1309943b198e00c5d86e71160a768c48e7f0"
}
```

Le bloc est celui du lanceur (forme d'archi). `paths` et `submodules` comme dans
[le cas de la garde des scripts](02-cas-de-campagne-la-garde-des-scripts.md) : `tools/rust_api` et le
sous-module `codeparsers` sont des dépendances par chemin que cargo lit même sans features. Sans features, le cas
ne demande pas la lib rag3db.

**Vérifié le 19 novembre à luciepc**, sur l'arbre archivé au parent avec le sous-module extrait, dans un target
propre : le témoin est rouge au parent (`0 passed; 1 failed`), vert avec notre correctif (`1 passed`).

Session mémoire (96), 19 novembre 2026, pour la campagne des correctifs passés. C'est un cas court, en Rust,
dans la boucle d'agent et le journal des runs du crate.

## Le point de départ

- **Commit parent** : `bcdac268f5` (19 novembre 2026). Le témoin y est, rouge ; le correctif n'y est pas.
- **Arbre** : `extension/rag3weaver`, avec `tools/rust_api` et le sous-module `codeparsers` à son commit.

## Le signalement, tel qu'il était

> Le code d'un refus du modèle n'est gardé nulle part.
>
> Le run `agent-1a1c02c5885-2122` compte les reprises de chaque appel (`retries: 4` sur l'appel 8) mais ne garde
> ni le code rendu par Vertex (429 RESOURCE_EXHAUSTED, 503, une coupure), ni l'heure ni l'attente de chaque reprise.
> `daemon.log` n'en dit rien non plus. Une seule trace indirecte : une connexion de l'hôte vers Google restée en
> CLOSE-WAIT pendant l'attente. On ne peut donc pas dire si le projet Vertex touchait son quota, ni à quel rythme.
>
> Ce qu'il faut : l'événement `LlmCall` du journal des runs porte la liste de ses reprises, avec la raison et
> l'attente de chacune.

(Le nom du projet Vertex, en clair dans le ticket, est remplacé ici.)

## Le témoin

- **Filtre** : `agent::tests::llm_call_event_keeps_each_retry_reason_and_wait`, dans
  `extension/rag3weaver/src/agent.rs`. Identique sur master.
- **Rouge au parent** : l'événement `LlmCall` sérialisé ne porte pas la liste des reprises avec leur raison et
  leur attente.
- **Suites exigées par les chemins** : aucune au parent pour ces fichiers. La lib du crate entière, sans
  features.

## Notre correctif (caché à Algareth, pour le juge)

`2d5b130994`, « feat(rag3weaver): l'événement LlmCall garde retry_log — la raison, l'attente et l'origine de
chaque réessai décidé du modèle » : `extension/rag3weaver/src/agent.rs` (+14 −2) et
`extension/rag3weaver/src/events.rs` (+19 −3).
