# Le modèle lit le texte de read_file, grep_files et list_files, pas leur enveloppe

Session tty, 19 novembre 2026, à la demande de c7 : l'hôte, mais l'expérience d'agent de code que Lucie demande. Archi, qui tient `src/backend_tools.rs`, a donné les règles. Ticket `docs/tickets/closed/2026-11-07-le-chat-envoie-au-modele-la-reponse-json-entiere-de-trois-outils.md`.

## Le défaut

`BackendTools::execute` donnait au modèle la `presentation` d'une réponse d'outil quand elle en avait une, celle d'une recherche. Sans elle, il donnait la réponse entière en JSON. Pour `read_file`, `grep_files` et `list_files`, le modèle lisait donc `delivery`, `validation`, `run` et un `metadata` vide autour du Markdown. Coupée à `max_tool_output_bytes`, cette chaîne arrivait comme un JSON invalide.

## Le correctif (`src/backend_tools.rs`, `model_text`)

- Sans `presentation`, quand `result` est du texte, le modèle lit ce texte, suivi des métadonnées non vides, sous la même forme que pour une recherche (« Metadata: … »). La ligne de fraîcheur y est déjà en tête : `dire_la_fraicheur` la pose dans `result`.
- L'enveloppe reste au journal du run.
- Tout le reste, et d'abord un échec, reste du JSON entier : `ok == false`, ou une `error` non nulle et non vide, même quand `result` est une chaîne (un graphe peut rendre un `result` partiel avec son erreur, et l'échec ne doit jamais disparaître derrière ce texte ; archi, 19 nov.). Une `presentation` reste lue même en échec : c'est celle qui dit un refus de validation. Trois lecteurs de l'hôte jugent l'échec par ce JSON : l'empreinte de lecture de `chat.rs`, `Step::note_tool` de l'équipier et le statut de `put_turn`. Un succès en texte libre y reste lu comme un succès.
- « — lines a-b of n » reste en tête du rendu de `read_file` : c'est le texte de `result` lui-même, et S2 comme `read_scope` le lisent.

## Les témoins

- `backend_tools::tests::the_model_reads_the_text_of_a_read_not_its_envelope`, rouge commité avant le code (df95d5b5e1 : le modèle recevait `delivery`, `validation` et `run` autour du Markdown), puis vert.
- `backend_tools::tests::a_presentation_stays_and_a_failure_stays_json`, vert avant comme après : il garde ce qui ne doit pas bouger.
- La garde de l'échec : rouge commité avant le code (253e92f98f : `{ok: false, error, result}` rendait le texte), puis vert.
- Le rejeu : par le pousseur de G, à luciepc (le témoin, les suites exigées, check_features, la lib entière).
