# Le rangement de l'hôte devient un mécanisme générique, déclaré

- **État** : ouvert, le 19 novembre 2026 ; pour archi, après le lanceur de la campagne et la chaîne des tranches, avec le lot B de mémoire.
- **Famille** : produit
- **Pour** : archi (l'hôte), mémoire (le gabarit, lot B)
- **Gravité** : forme : l'hôte porte en Rust ce qui devrait être une déclaration. Un agent ou une personne ne peut pas refaire ce DAG comme un autre.
- **Atteignable en service** : oui, chaque compaction et chaque tranche passe par ce code.
- **Touche rag3weaver** : oui, `src/daemon_host/rangement.rs`, `src/daemon_host/chat.rs`, `src/llm.rs`
- **Signalé par** : Lucie, par c7, le 19 novembre 2026, sur la réponse d'archi « l'archiviste côté hôte »

## La décision de Lucie

« tout doit être déclaratif en général ; quelqu'un doit pouvoir se dire : c'est juste un DAG comme un
autre, j'aurais pu en faire un moi-même ; et pareil pour des boucles auto-amélioratives plus tard, un
agent qui édite son propre workflow de mémoire »

## Ce qui est en dur aujourd'hui

- **Les deux textes du tour de reprise** (chat.rs, `compacter`) : « [Context restored after a
  compaction: …] » et « Context restored. ».
- **Les arguments de `compose_restart`** : le budget (la moitié de la fenêtre du modèle), et
  `identity` et `task_fiche` passés vides.
- **Le déclencheur** : un seuil de jetons depuis la marque (`memory.slice_tokens`, comptés en
  ceil(caractères / 4)), et la compaction à `memory.compact_at`. Ce pourrait être, demain, « fin de
  sujet » ou « inactivité ».
- **Les noms des outils appelés** (`put_turn`, `archive_slice`, `compose_restart`, `cited_by`) et
  l'ordre des appels.
- **La reconnaissance d'un dépassement de contexte** par le texte « llm context overflow » dans
  l'erreur.

## Ce qu'il faut faire

1. **L'hôte ne garde qu'un mécanisme générique** : « appeler tel outil déclaré, à tel moment
   déclaré, avec tels arguments déclarés ». Il n'a aucun texte ni aucun nom propre à l'archiviste.
   Les moments : la fin d'un tour, un seuil, le début d'un tour, un dépassement. Les arguments :
   des ingrédients (la tranche, les sources, la fenêtre).
2. **Le gabarit memory**, ou le manifeste, **déclare le rangement** : quels outils, à quel moment,
   avec quels textes et quels budgets (`restart.context.*`, `archivist.context.*`, lot B de
   mémoire).
3. **Le dépassement de contexte devient une erreur typée** du client de modèle
   (`LlmError::ContextOverflow` jusqu'à l'appelant), et non plus un motif lu dans un texte.

## Comment le prouver

- Les témoins actuels du rangement restent verts : chat::tests, `a_slice_is_filed…`,
  `a_compaction_…`, `a_turn_that_overflows…`.
- Un nouveau témoin montre un gabarit de test qui déclare d'autres outils et d'autres textes : l'hôte
  les appelle sans une ligne de Rust en plus.
- `grep` ne trouve plus aucun texte de reprise ni aucun nom d'outil de mémoire dans
  `daemon_host/rangement.rs` et `chat.rs`.

## Taille et blocage

2 jours, côté hôte. Il attend le lanceur de la campagne, la chaîne des tranches et le lot B de
mémoire.
