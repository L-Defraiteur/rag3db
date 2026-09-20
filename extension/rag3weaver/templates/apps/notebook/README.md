# Chat générique : terminal et navigateur

Cet exemple utilise le même agent Rust dans les deux interfaces. Il fournit l'export de fichiers ; brancher un backend pour ajouter ses outils de recherche et de persistance.

Depuis la racine de rag3db :

```bash
cargo build --offline --manifest-path extension/rag3weaver/Cargo.toml \
  --features openai-llm --bin rag3weaver-chat -j 2
python3 extension/rag3weaver/scripts/chat_app.py \
  extension/rag3weaver/templates/apps/notebook/chat.json --demo
```

Le lien affiché ouvre la page web ; le TUI reste actif dans le terminal. `Ctrl-Q` ferme les deux. Ajouter `--web-only` pour ne lancer que le serveur. Le mode démo affiche une réponse fixe et ne démarre aucun backend.

Pour utiliser un modèle réel, adapter `llm` dans `chat.json` puis retirer `--demo`. La clé optionnelle est référencée par `api_key_env`. Les données sont dans `data/`, ignoré par Git. `backend_command` permet de brancher un hôte déclaratif, et `allowed_tools` d'en limiter les outils.

Voir la [configuration, l'architecture et les limites](../../../docs/20-09-2026/17-agent-generique-tui-web-et-providers.md).
