# Les rejeux de correctifs de la nuit

21 novembre 2026, de 0 h 30 à 1 h, à la tablette. La consigne de Lucie (relayée par c7 le 20 au soir) : « des scénarios qui
reproduisent vraiment des correctifs que vous aviez faits ». L'équipier, lancé par archi, rejoue un ticket fermé de la
semaine depuis le commit parent de son lot, sur une copie du dépôt, avec le ticket pour seule consigne. Le banc juge le
résultat rendu, jamais l'intention :

- **le rouge** : les témoins au parent ;
- **le diff de l'équipier** appliqué sur le parent : les témoins et la suite du lot, verts ou non ;
- **contre le correctif atterri** : mêmes fichiers, même intention, l'écart dit ; et la même suite jouée sur le commit
  atterri, pour référence.

Le juge des cas Rust est `~/.cache/rag3weaver-build/juge-rejeu.sh <dossier du cas>` à la tablette, et ses sorties sont dans
`~/.cache/rag3weaver-build/banc-parite-tablette/rejeux/`. Chaque cas a son worktree sous `~/git_workspaces/banc/`.

## Le relevé

| cas | rouge au parent | le diff de l'équipier | contre l'atterri | verdict |
|---|---|---|---|---|
| A, la réponse coupée refusée (mémoire) | oui, les 2 témoins | **ne compile pas** | 3 fichiers sur 5 | **échoué** |
| B, le bac à sable sans sccache | oui (lanceur, sur l'assertion) | **vide** | — | **échoué** |
| C, la chaîne Rust du bac pour lucivy | oui (code 1) | **vert** (code 0) | le même fichier, la même intention | **réussi** |

## A, la réponse coupée refusée (mémoire)

- **Le cas** : refuser une réponse coupée par `max_tokens` sous `response_schema` en le disant, et déclarer `reasoning`
  sur `LlmNode`. Parent `0766144818`. Témoins : `dataflow::llm_nodes::tests::a_reply_cut_by_its_limit_is_refused_saying_so`
  et `the_declared_reasoning_reaches_the_model`. Suite : `cargo test --lib -- dataflow::llm_nodes::`.
- **Au parent** : les deux témoins rouges, la suite à 22 passés et 2 rouges (les témoins).
- **Le diff de l'équipier** (369 s ; 3 fichiers, +41 −4, aucune ligne de test touchée) : **le crate ne compile pas**.
  `llm_nodes.rs:283`, `let max_tok = self.opts.max_tokens.unwrap_or(0);` : E0599, `unwrap_or` sur un `usize`.
  L'équipier a pris `max_tokens` pour une `Option`.
- **Contre l'atterri** : il touche `llm_nodes.rs`, `archive_slice.mmd` et `archive_slice_core.mmd`, comme l'atterri ; il
  lui manque `llm.rs` (la fonction partagée `cut_reply`) et `archive_segment.mmd`.
- **Une faute du cas, corrigée par archi** : la référence nommée d'abord, `4810e0faee`, **ne compile pas seule**. Dans
  `llm.rs`, `cut_reply` y est insérée entre `#[derive(…)]` et `pub struct Usage` (E0774). Le commit suivant,
  `f935412ab5` (« cut_reply après la structure Usage »), la répare : c'est lui qui fait foi. Sur lui, les deux témoins
  passent ; la suite garde un rouge, `the_usage_of_a_call_comes_out_on_its_port`, le témoin rouge du lot suivant (le port
  `usage`), pas un défaut de ce lot. **Master contient donc un commit qui ne compile pas** : un `git bisect` y trébuchera.

## B, le bac à sable sans sccache

- **Le cas** : une commande lancée dans le bac à sable ne doit pas hériter des enveloppes de compilateur de l'hôte
  (`RUSTC_WRAPPER`…). Parent `81ccf05db4`, correctif atterri `20d406a82c` (`commande.rs`, +6 −2). Témoin :
  `commande::tests::a_compiler_wrapper_of_the_host_does_not_reach_the_sandbox`, rouge au parent sur son assertion
  (lanceur, `red.json`).
- **Le tour** : 509 s, 3 pas, 78 appels au modèle, puis « 3 pas de suite sans élément réussi : je m'arrête ». **Diff vide.**
  Le diagnostic était juste dans la boîte (la liste `variables_du_poste_hors_du_bac` laissée vide, le témoin nommé), puis
  l'équipier est parti dans `scripts/poste` et le script du poste, que le ticket cite, et n'a rien écrit.
- **Verdict : échoué.** Un diff vide laisse le témoin rouge.

## C, la chaîne Rust du bac pour lucivy

- **Le cas** : le bac à sable de la campagne reçoit les vrais `RUSTUP_HOME` et `CARGO_HOME`, pour un arbre sans fichier de
  chaîne (lucivy). Parent `6dfc72b6f0`, correctif atterri `e3f6e10313` (`tools/campagne/lanceur.py`, +6). Témoin :
  `tools/campagne/test_lanceur.py`, jugé par son code de sortie.
- **Au parent** : code 1 (10 tests, 1 en erreur). **Avec le diff de l'équipier** (156 s) : **code 0**, 10 tests verts.
  **Sur l'atterri** : code 0.
- **Contre l'atterri** : le même fichier, la même intention (`env["RUSTUP_HOME"]`, `env["CARGO_HOME"]` vers le vrai
  dossier personnel). **Deux écarts** :
  - l'équipier crée la section `sandbox` quand elle manque (`setdefault("sandbox", {})`), alors que l'atterri ne
    touche qu'un bac déjà déclaré (`if "sandbox" in m["workspace"]`). Un manifeste sans bac en reçoit donc un, vide ;
  - son diff porte deux fichiers `tools/campagne/__pycache__/*.pyc` (du binaire de Python), du déchet à ne pas commiter.
- **Verdict : réussi**, avec ces deux écarts dits.
