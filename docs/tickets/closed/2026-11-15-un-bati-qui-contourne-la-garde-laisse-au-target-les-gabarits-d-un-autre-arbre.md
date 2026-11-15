# Un bâti qui contourne la garde laisse au target les gabarits d'un autre arbre

- **État** : fait (15 novembre 2026, 86545785c puis la sortie périmée retirée) : la garde lit les sorties des build scripts, efface les empreintes quand un rerun-if-changed vise un autre worktree, même sous le bon marqueur, et retire la sortie périmée (sinon une unité que personne ne rebâtit ferait tout rebâtir à chaque passe) ; sept cas neufs au témoin, rouges d abord ; delais-c trouvé pris, archi prévenu
- **Famille** : produit (outillage : `scripts/garde_target.sh`)
- **Gravité** : faux rouge, ou faux vert. Une passe tourne sur les gabarits d'un autre worktree sans le dire.
- **Atteignable en service** : non (les targets partagés des passes)
- **Touche rag3weaver** : oui (`scripts/garde_target.sh`, `build.rs`)
- **Pour** : G (optimiseur)
- **Signalé par** : session archi, 15 novembre 2026, vers 17 h 50

## Ce qu'on voit

Sur wt-pas, trois témoins du chat ont rendu « compose_restart : builtin:backends/memory/plans/compose_restart.json :
aucun gabarit livré de ce nom », alors que le fichier était dans l'arbre. Le target `delais-c` était partagé avec
wt-branchement (la tête d'ED d'avant le lot de 96), sous `RAG3WEAVER_TARGET_REPRENDRE=1`. Un `touch build.rs` l'a remis
d'aplomb.

## La cause

`build.rs` embarque les gabarits par `include_bytes!` sur des chemins absolus, dans le `OUT_DIR` du target. Il dit
`cargo:rerun-if-changed=<arbre>/templates`, un chemin absolu lui aussi. Cargo ne met pas le chemin d'un paquet local
dans son hachage : deux worktrees partagent donc le même `OUT_DIR`.

La garde efface les empreintes des paquets locaux quand l'arbre change, y compris celle du lancement de `build.rs`.
Mais l'étape « module » des passes d'archi lançait `cargo test --lib` en direct, sans la garde. Le marqueur
`.arbre-du-target` gardait alors l'arbre d'avant. Au retour sur cet arbre, la garde voyait « même arbre » et n'effaçait
rien. Cargo jugeait `build.rs` à jour, puisque les chemins qu'il surveille sont ceux de l'autre arbre, qui n'avaient pas
bougé. Le `OUT_DIR` gardait ses `include_bytes!`.

## Ce qu'il faudrait

Que la garde lise le fichier `output` des build scripts locaux du target. Si un chemin `rerun-if-changed` est sous un
autre worktree du dépôt que l'arbre, elle efface les empreintes des paquets locaux et le dit nommément, même quand le
marqueur est le bon. Cela ferme aussi le cas d'un bâti qui l'a contournée. Le témoin, dans `scripts/test_garde_target.sh`,
bâtit sous deux arbres et vérifie que le `OUT_DIR` pointe sur le second.

De son côté, archi fait passer l'étape « module » par `garde_target.sh --reprendre`, ce qui ferme son cas, pas le cas
général.
