# Un second shutdown annonce fermé après un arrêt échoué

- **État** : corrigé (`7f2a0f65e`, 11 novembre 2026) : résultat de tentative mémorisé ; passe complète de la tête finale encore requise avant intégration.
- **Famille** : produit
- **Gravité** : faux succès de fermeture, alors qu'une réaction détachée peut encore tenir le catalogue.
- **Atteignable en service** : oui via l'API `Backend::shutdown` appelée deux fois après une erreur ; le chemin `DaemonHost::close_slot` propage la première erreur puis retire le backend.
- **Touche rag3weaver** : oui (`src/backend.rs`, état d'arrêt et Drop).

## Cause et contrat

Le lot d'arrêt pose `ferme=true` avant `stop_reactions`. Si une réaction dépasse la borne, ses handles sont consommés et sa tâche détachée ; `shutdown` rend une erreur et laisse volontairement le catalogue ouvert. Un second appel voit le booléen et rend immédiatement `Ok(())`. Ce booléen est documenté comme une fermeture acquise, pas comme une tentative unique. Le résultat observé contredit le contrat.

Réessayer aveuglément n'est pas sûr : le registre des réactions ne tient plus la tâche détachée, qui peut encore utiliser le catalogue. Conserver le résultat terminal de la tentative permet de restituer l'erreur sans annoncer une fermeture ni fermer sous cette tâche.

## Rouge observé

Témoin seul `8c426ad3d`, branche `cx-recherche-arret-audit`, worktree `cx-rag3db-arret-audit`. À luciepc sous `poste lourd`, `CARGO_BUILD_JOBS=8`, lib complète 29a7bc52a, `RAG3WEAVER_STOP_WAIT_S=1` pour ce processus :

`cargo test --lib --features rag3db-native,burn-embedder,burn-ocr,code,daemon,postgres,openai-llm,anthropic-llm,navigateur un_arret_echoue_ne_devient_pas_un_succes_sans_fermer -- --nocapture`

Journal `~/.cache/rag3weaver-build/cx-recherche-arret-audit.log` : premier `Err` avec « catalogue laissé ouvert », second `Ok(())`, vue encore ouverte ; assertion « faux succès » rouge, 0 réussi/1 échoué/0 ignoré, 1,01 s. La réaction est libérée par son canal avant les assertions : aucun fil figé ne reste volontairement.

## Pour fermer

Remplacer l'état ambigu par un résultat d'arrêt mémorisé, ou un état explicite équivalent. Un échec doit rester observable sur les appels suivants et au Drop ; un succès reste idempotent. Jouer le témoin vert, les témoins normaux d'arrêt, puis les contrôles requis sur la nouvelle tête. Ne pas présenter la passe R3 antérieure comme validation de ce correctif.

## Correctif et vérifications

`Option<Result<(), String>>` conserve la tentative unique d'arrêt, succès ou erreur. L'accès est exclusif par `&mut self` ; aucun AtomicBool n'est nécessaire. Un échec reste rendu aux appels suivants et au Drop, sans retenter sous une réaction détachée. Sur `7f2a0f65e`, les deux ciblés sont verts (2 réussis/0 échec/0 ignoré, 1,01 s), journal `cx-recherche-arret-audit-vert.log` à luciepc : première et seconde erreurs identiques, vue encore ouverte ; après succès, deux appels et le Drop n'émettent qu'un ShutdownStarted/ShutdownCompleted.

`545fbfd17` isole le réacteur volontairement figé dans un processus enfant avec délai d'une seconde, afin de ne pas tenir un worker du runtime partagé pendant la lib entière. La passe de la tête intégrant cette isolation reste à lire ; la R3 d'avant le correctif est historique et ne la remplace pas.
