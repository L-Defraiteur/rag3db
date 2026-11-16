# Microfix cfg(test) : racine disque et unicité, compilation non jouée

Base union5f59/masterb0 conservée. Ticket repris a8d317b251 AVANTdiff ;
trois témoins de chemin publiés6b30e771e7 AVANTimpl5c649d4b45.
Vrai rouge absentROOT publié b414d84d27, ROHotfix23SHA indépendants.
Les trois nouveaux témoins ne sont pas annoncés vus rouges ni verts : NONJOUÉS.

Seul Rust changé : code_sync_fts_guard_double_tests.rs, déjà sous cfg(test).
Cinq juges natifs/corps/données/oracles inchangés. ROOT explicite existante,
absolue/canonique/répertoire et tout chemin sans symlink, erreur sans fallback.
ROOTabsente : CARGO_TARGET_DIR standard (relatif résolu depuis manifest), sinon
CARGO_MANIFEST_DIR embarqué/target, sous-parent fts-double-fixtures ; aucun
HOME ou /tmp implicitement choisi. Création parent700, puis primitive
existante tempfile::Builder::tempdir_in(root).keep() comme chat_qui_code :
enfant unique même label/ROOT, contenu conservé jusqu'après Catalog et au-delà.
Aucun destructeur TempDir avant DB. Trace FTS_DOUBLE_CASE publie le chemin.
Validation séquentielle de chemins, pas résistance aux renommages malveillants
concurrents prétendue. Un target explicitement fourni en RAM reste choix appelant.

Trois témoins filesystem supplémentaires sans mutation d'environnement :
ROOT explicite répétée garde sentinelle et aucun target fallback ; ROOT absente
reste sous target privé ; racines relatives/manquantes/symlinks refusées.
Soit8fonctions inventoriables (5natives+3chemins), aucunejouée. Rustfmtparse
et diffcheck seuls ; pas Cargo/rustc/build/native/DB/provider/service/targetcréé.

Recette ciblée à publier séparément avant tout nouveau GO : source propre figée,
cacheBC strict sous vrai poste32G/unique1200 ; build900 sourceJSON/code+native,
ELFcopieSHA+inventaire8/0ignored+ledger avant runtime, 3chemins exacts puis
5natives (premierGDBmainmêmeappel), puis GENsansROOT et2GENmêmeROOT, caps60,
contrôle chemins uniques/anciensbytes conservés/profilprivé, finalquiescence
et hashes+deadline. Total8juges distincts/11invocations, pas8nouveauxjuges.
Aucun rejeu lib entière/F8/NAV/canonique ni GO ancien repris. GlobalHOLD ;
source correcte n'est pas compilation prouvée. Recomposition globale après
ciblés verts seulement selon C7, sans inventer intégration master.
