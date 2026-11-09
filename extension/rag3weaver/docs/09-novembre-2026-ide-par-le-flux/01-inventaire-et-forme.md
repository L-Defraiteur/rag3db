# L'IDE par le flux : l'inventaire, puis la forme

Session archi, 9 novembre 2026. C'est une demande de Lucie, du 8 au soir : « que ces trucs-là
fonctionnent par SSE, pas par polling, voire rendre générique quand on a besoin d'un SSE ». c7 me
l'a confiée le 9.

## Ce qui interroge l'hôte (relevé sur master `ab8272deb`)

**Au repos, rien.** Aucune minuterie de l'IDE n'appelle l'hôte : celle de `lib/progres.ts` ne
fait que re-rendre la page, pour l'âge d'un avancement figé. Le témoin l'a mesuré : une minute
d'inactivité sur la page du chat, zéro requête hors du flux.

Les relectures viennent des **gestes** et des **événements**. Un événement arrive par le flux, et
la page relit alors l'état à l'hôte au lieu de le recevoir avec l'événement.

| où | déclencheur | ce qui est relu | combien |
|---|---|---|---|
| `Chat.tsx`, fin d'un tour envoyé d'ici | la réponse du `POST /ide/chat` | `GET /ide/chat` (la session) | 1 |
| `Chat.tsx`, `reloadRecaps` en fin de tour | idem | `GET /ide/chat` pour **chaque** conversation de la barre | N |
| `Chat.tsx`, l'usage du tour | idem | `GET /ide/chat`, toutes les 300 ms, jusqu'à voir le run au journal | jusqu'à 10 |
| `Chat.tsx`, un tour venu d'ailleurs (la carte d'accord) | `RunFinished` | `GET /ide/chat`, toutes les 100 ms, jusqu'à ce que le run soit levé | jusqu'à 50 |
| `main.tsx` | `RepoAttached`, `RepoIndexed` | `GET /ide/etat` (tout l'état de l'hôte) | 1 par événement |
| `Propositions.tsx` | sujet `declarations` | `GET /ide/propositions` | 1 par événement |
| `File.tsx` | `FileChanged` du fichier ouvert | le fichier relu, après 300 ms de calme | 1 par rafale |

Il y a aussi des lectures de chargement, légitimes : l'état et les récapitulatifs à l'ouverture,
la carte d'accord à son montage, la relecture du chat quand le flux se rouvre.

**Ce que ça coûte.** Le journal de l'hôte ne trace pas les requêtes HTTP : il ne peut pas dire le
coût par minute, et le témoin le mesure à sa place. Pour un tour, la page de test fait trois
`GET /ide/chat` après son `POST`. Chez Lucie, avec N conversations dans la barre, c'est 1 + N +
jusqu'à 10 relectures par tour. Pendant un index, chaque `RepoIndexed` relit l'état entier de
l'hôte.

## La forme

- **Côté hôte, un état publié.** `ide_flux().publier_etat(sujet, cle, etat)` rend un événement
  `{ kind: "Etat", cle, etat }` sur le sujet. Celui qui change l'état le publie, entier ou par
  ligne, au moment où il change.
- **Côté IDE, une seule façon de s'abonner.** `useSujet(sujet, cle, lireAuChargement)` lit une
  fois au chargement, puis garde l'état que le flux pousse, sans autre requête. Les vues le
  réutilisent.
- **Dans ce lot :**
  - **La fin d'un tour** : l'hôte publie, sur le sujet `chat` et la clé du backend, le
    récapitulatif (contexte, usage, modèle, fiches écrites) et les tours ajoutés. Il le fait
    quand la session est reposée et l'usage écrit au journal. La page n'a plus de relecture ni
    d'attente, que le tour vienne d'elle ou d'ailleurs.
  - **L'état d'un dépôt** : `RepoAttached` et `RepoIndexed` portent sa ligne, que la page met à
    jour dans l'état sans relire tout l'hôte.
- **Lot suivant**, au même moule : les propositions (le sujet `declarations` porte la liste
  changée) et le fichier ouvert (`FileChanged` porte le contenu s'il est petit).

## Le témoin

`daemon_host::ide_par_le_flux::the_ide_asks_the_host_nothing_while_idle_and_only_its_gesture_for_a_turn`
enveloppe `fetch` dans la page. Il attend :
- une minute d'inactivité : rien ;
- un tour, puis cinq secondes : le seul `POST /ide/chat`.

Rouge le 9 novembre : `["POST /ide/chat", "GET /ide/chat", "GET /ide/chat", "GET /ide/chat"]`.
