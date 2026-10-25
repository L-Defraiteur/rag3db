# Everything declarative — ce que le 25 octobre 2026 a appris

Le relevé des 11, 12, 18 et 19 octobre est dans `../11-octobre-2026-everything-declarative/`.
Celui-ci ajoute ce que la journée a appris.

## Le flux de l'IDE (`src/ide.rs`)

- `Flux::publier` numérote chaque événement et le garde dans un tampon (2 048) pour le rejeu.
  `Flux::abonner` écrit l'en-tête SSE et inscrit l'abonné **sous le même verrou**. Un `onopen`
  côté page prouve donc que les publications suivantes arriveront.
- `Flux::relayer` : **un fil par sujet** (`ide-relais-<sujet>`), en `recv_blocking`. Il finit
  quand le bus se ferme ou que le flux n'est plus. Rien ne doit repasser par
  `dataflow::rt::runtime()` : ce runtime n'a que **deux fils pour tout le processus**, et les
  nœuds qui bloquent les tiennent.
- Les sujets sont relayés chacun à part : **aucun ordre entre sujets**. Une page qui attend un
  fait d'un sujet après un autre (une proposition après la fin d'un tour) doit le tenir d'une
  source ordonnée, comme la réponse HTTP, ou accepter l'événement tardif.
- La page de l'IDE est `src/ide/page.html` (les données : le bloc `outils`) plus
  `src/ide/page.js` (le code, servi par `GET /ide/page.js`). L'atelier suit la même forme
  (`/_atelier/page.js`). `ide::tests::the_page_only_calls_what_the_code_template_serves` lit le
  bloc dans la page et les appels dans `page.js`. Tout outil appelé par `appeler(id, "…")` doit
  être dans le bloc.

## Le chat de l'atelier dans la page

- La bulle d'un tour porte `data-run`, posé dès `RunStarted` ou par `relireLeChat`. Un événement
  d'outil va à la bulle de son run, même finie. Les jetons ne vont qu'au tour en cours : la
  réponse pose le texte entier.
- `relireLeChat()` est appelé à chaque `onopen`. Il ne prend le run que pendant un tour que la
  page attend (la bulle `en-cours`). `window.__ide = { relireLeChat, flux }` est là pour les
  témoins.
- La réponse de `POST /ide/chat` porte `propositions` (id, paths, accepted), lues dans les tours
  d'outil `propose` de ce tour-ci.

## Le rendu d'un Plan (`RenderResultsNode`)

- `json = results | rows` (défaut results) et `provenance`. En format json avec json=rows : `text`
  porte un **objet** `{rows, total, provenance}`. Le résultat d'un outil reste alors un objet, et
  `Backend::dire_la_fraicheur` ne préfixe que les résultats textuels.
- En markdown, rows est lu et laissé ; `total` et `provenance` vont dans la vue du gabarit.
- `verifier_le_cablage` est appelé par le chargeur du manifeste et par le test des outils
  livrés. Il ne juge que si le format peut valoir json (écrit, ou `$…`). Une clé liée attend
  l'exécution. En json=rows, aucune arête ne doit partir de `text`.
- `outline` (gabarit de codeparsers) : `format=markdown|json` ; la marque est sa `provenance`,
  mot pour mot, avec l'apostrophe droite. Dans le Mermaid, une valeur entre guillemets doubles
  marche dans une étiquette, puisque le parseur cherche le dernier `"]`.

## Les témoins sous charge

- Un rouge qui n'arrive que dans la lib entière, jamais au témoin seul, même sous la charge d'un
  autre processus, désigne une **ressource partagée dans le processus**. Ici, c'était le runtime
  à deux fils.
- La preuve d'une course se fait par une sonde déterministe qui force le mauvais ordre : une
  fabrique ralentie, ou des fils tenus. « 0 rouge sur 10 en essayant plus vite » ne prouve rien.
  Annuler à 0 ms ne reproduisait pas la course ; la sonde de la fabrique, si.
- Une étape d'un témoin navigateur est bornée **après un fait de l'hôte**, et son message dit
  que l'hôte a fait sa part.

## Le magasin et les passes à luciepc

- `RAG3DB_LIB=<identifiant complet>` est résolu par `lib_du_magasin`, planchers vérifiés.
  `run_e2e.sh` dit « moteur lié : … » ; la lib entière, via `I-court-reprise.sh`, dit « lib du
  magasin (…) » dans `I-luciepc/reprise/lib.log`.
- La garde du target refuse au-delà de 20 Go (`RAG3WEAVER_TARGET_MAX_GO`). Un target rebâti
  pèse environ 5,5 Go.
- Le shell distant est fish : tout ce qui a des boucles ou `$(…)` passe par un script bash
  copié puis lancé.

## Le juge, le journal, l'équipier (le soir)

- **Un outil ne peut pas arrêter la boucle qui l'appelle** (`agent.rs`) : seuls `pause_dialogue`
  et `confirm_pause` étaient interceptés. `Agent::with_terminal_tools` déclare des noms dont
  l'appel réussi finit le tour (`StopReason::Terminal { tool, arguments, result }`) ; la boucle
  sort à la fin du tour, l'historique reste rejouable. `ChatRunOptions.terminal_tools` le porte
  jusqu'au chat de l'hôte.
- **`recall_apports` et `recall_fiches` cherchent par le mot** : une fiche « outil : read_file »
  ne répond pas à la requête « bilan ». La vue des bilans interroge par « bilan » puis par
  « outil », et les apports par le nom de leur fiche.
- **`get_fiche` répond même quand la fiche n'existe pas** : elle existe si la réponse porte son
  nom (JSON échappé).
- **Une tâche est un worktree du HEAD du dossier du backend** (`code-tree add` en fait un dépôt
  et commite). Un manifeste modifié et non commité ne suit pas : une clé `bilans` ajoutée à la
  main doit être commitée avant d'ouvrir la tâche.
- **Vertex refuse deux déclarations d'outil du même nom** (« Duplicate function declaration »).
  `propose` est l'outil des propositions de déclaration ; celui du hors-plan de l'équipier
  s'appelle `suggest`, et les outils du pas filtrent les homonymes du backend.
- **Le journal des runs d'un équipier porte `demande`** : sans elle, le juge nommerait la fiche
  de session par le premier message de l'agent, qui est la frontière du pas — une fiche par pas.
- **Le `Cargo.lock` du crate est dans le dépôt** : une dépendance directe ajoutée (zstd) y écrit
  une ligne, à commiter avec le lot, sinon la passe de push s'arrête sur « arbre sale ».
- **zstd sur de petits JSON** : ×2 à ×4, pas ×5 à ×10. Un run ordinaire pèse 1 à 4 Kio.
- **`RhaiNode` inline** : `script='…'` avec `input` comme variable d'entrée ; pas de guillemets
  doubles dans un script inline (le `.mmd` les tient), les accents graves de Rhai (`\`main.rs\``)
  passent.
