# Cinq fichiers atteignent la borne du banc public

- **État** : ouvert — cinq délais de dix secondes observés, aucun retry
- **Famille** : produit
- **Gravité** : parsing non observé jusqu'à son terme pour ces fichiers du corpus
- **Atteignable en service** : à déterminer ; la borne inclut aussi lecture, hash, initialisation et collecte du banc
- **Touche rag3weaver** : potentiellement, extraction codeparsers ; aucune ingestion ici
- **Pour** : codeparsers, diagnostic ciblé après baseline sous allocation neuve
- **Signalé par** : campagne FULL02b achevée, source384165760b / parser1b35cfb / ELFbff1cafb

## Fichiers observés

Les cinq lignes ont execution_status=timeout, worker_returncode=-9 après arrêt
du groupe propre par le contrôleur. Cela ne prouve pas un OOM ni dix secondes
CPU du parseur. Les octets source sont figés par Banc ; aucun projet public exécuté.

- Séquence5578 : `github.com/dotnet/roslyn@c6dae7dd4f51cef1680c6b23ab614394d6989a69`, `src/Compilers/VisualBasic/Test/Semantic/Binding/T_1247520.cs`, 787111 octets ; SHA256 `924579c01284e0020f121e3d80592d3f14b960078bf2de495546811f2de29847`.
- Séquence20883 : `github.com/rust-lang/rust-analyzer@15d97b98eacfad690e81757e867ca0800f1c9e07`, `crates/hir/src/lib.rs`, 231120 octets ; SHA256 `3ce10e550720d2d1d3c8152acea0c568c46e4f05a5bd31729f84bcc2093bd0f5`.
- Séquence28613 : `github.com/microsoft/vscode@6e68549edb69380c9316e49df5d3795d5064a2b0`, `src/vs/platform/agentHost/test/node/claudeAgent.test.ts`, 588495 octets ; SHA256 `81b78deea19efab8bba474cd5bdcee37eb3951002896caffdca766efaa71838f`.
- Séquence29069 : `github.com/microsoft/vscode@6e68549edb69380c9316e49df5d3795d5064a2b0`, `src/vs/workbench/contrib/chat/browser/widget/input/chatInputPart.ts`, 307069 octets ; SHA256 `c8667e7d6186189d231ad853f0df1e9baa63dfcb731a7fc3725b55b90aa6f626`.
- Séquence29325 : `github.com/microsoft/vscode@6e68549edb69380c9316e49df5d3795d5064a2b0`, `src/vs/workbench/contrib/sessions/browser/views/sessionsList.ts`, 527289 octets ; SHA256 `9f96b488f6ff15bd2bf95fb5eb8ca1ede20a4f13fa8a0deae6de3d2398e1558e`.

## Preuve et travail restant

32382 résultats conservés aux deux postes dans
`~/.cache/codeparsers-notes/language-corpus-02b/campaign/results.jsonl`, SHA
0667eab18d7892b442b8b912d98528d1253041620f43c60a286b47cd5fa81678.
Aucune relance de ces fichiers pendant la campagne. Le worker a été remplacé,
les fichiers suivants ont continué, la borne globale a été respectée.
Localiser les coûts sous budget neuf sans exécuter les dépôts, puis décider si
le contrat impose une correction, une borne adaptée ou une limite documentée.
Ne pas appeler les cinq cas un même défaut de parseur sans ce diagnostic.
Parent : `2026-11-16-qualifier-la-couverture-sur-des-corpus-publics-par-langage.md`.
