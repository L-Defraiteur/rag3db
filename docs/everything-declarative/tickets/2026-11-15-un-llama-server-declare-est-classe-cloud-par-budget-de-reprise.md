# Un llama-server déclaré est classé cloud par `budget_de_reprise`

- **État** : ouvert (15 novembre 2026)
- **Famille** : produit
- **Gravité** : coût. Un modèle local reçoit le budget de reprise d'un modèle cloud (100 000 jetons) au
  lieu de la moitié de sa fenêtre ; sur une petite fenêtre, la compaction repartirait au-dessus de ce
  qu'elle tient
- **Atteignable en service** : oui. Tout llama-server déclaré dans `models.llm` sans
  `restart_budget_tokens` explicite
- **Touche rag3weaver** : oui (`src/model_source.rs`, `ModelSource::budget_de_reprise`)
- **Pour** : everything declarative (ED)
- **Signalé par** : ED, en écrivant le bloc `models.llm` du manifeste de Lucie pour c7 (15 novembre
  2026, 18 h)

## Ce que c'est

`budget_de_reprise` (master f9e894f36) reconnaît un modèle local par `provider` `local` ou `service`, ou
par `protocol: "llama_server"`. Or `connect_llm` refuse `llama_server` pour un modèle de langage : un
llama-server se déclare `provider: "compatible"`, `protocol: "openai"`, avec une adresse
`http://127.0.0.1:8080/v1`. Déclaré ainsi, correctement, il est classé cloud.

## Le témoin

Rouge d'abord : un modèle `compatible`, `openai`, à l'adresse `http://127.0.0.1:8080/v1`, avec
`context_tokens` 32 768 et sans `restart_budget_tokens`, a un budget de reprise de 16 384. Le même à une
adresse publique a 100 000.

## Ce qu'il faut pour le fermer

Reconnaître le local aussi par une adresse de boucle locale (127.0.0.1, localhost, ::1). Retirer
`llama_server` des protocoles locaux, puisque `connect_llm` ne l'accepte pas, ou l'y accepter. Faire
passer le témoin du rouge au vert.
