# L'affichage en direct du chat peut manquer le premier jeton du premier tour, sous charge

- **État** : corrigé (7 novembre 2026, hooks-essai, lot premier-jeton) : la cause était dans le flux de l'IDE, pas dans la page — `Flux::publier` tirait l'id hors du verrou des abonnés, et `Abonne::envoyer` sautait une trame quand deux relais se croisaient (N+1 envoyé avant N) ; l'id se tire désormais sous le verrou, avec l'empilement et l'envoi. Témoins : `ide::tests::relays_publishing_together_lose_nothing_and_keep_the_order` (rouge : 7 577 reçus sur 8 000) et `le_premier_jeton_d_un_tour_arrive_dans_la_page` (rouge 3 fois sur 3, un jeton absent dès la réception, aucun EventsMissed), verts 3 fois sur 3 ; la relecture de fin de tour reste la source de vérité
- **Famille** : produit
- **Gravité** : réponse fausse passagère à l'écran (une réponse en cours qui commence tronquée,
  corrigée à la fin du tour).
- **Atteignable en service** : oui, sous charge, au premier tour après l'ouverture de la page.
- **Touche rag3weaver** : oui (`ui/ide/src/views/Chat.tsx`, le flux de l'IDE `src/ide.rs`).

## Ce que c'est

Avec la page du chat en parties (lot B, 1b8a4476a), le premier jeton du premier tour envoyé après
l'ouverture de la page manquait parfois à l'écran : « liste » au lieu de « je liste », un Markdown
sans son `**` d'ouverture. La réponse finale restait tronquée.

## Les chiffres (luciepc, lib 04063ee06 épinglée, 7 novembre 2026)

| page | passes | rouges |
|---|---|---|
| l'ancienne (3746f13fb) : `le_markdown_…`, `le_chat_se_mene_…` | 3 | 0 |
| en parties (1b8a4476a) : les mêmes et `la_page_du_chat_…` | 4 | 2 |
| en parties (3d8701b51) | 3 | 2 |
| en parties, la session fait foi en fin de tour (6d3504d56, 743907807) | 6 | 0 (pour ce défaut) |

Même verte, la page en parties prenait environ 6 s pour ces témoins, contre 2 s pour l'ancienne.

## Ce qui est écarté (à la lecture)

- L'abonnement du flux de l'IDE au bus d'un backend : il se fait à l'ouverture, avant tout appel
  (`DaemonHost::open_slot` → `IdeFlux::relayer`).
- Le filtre sur `runRef` dans le gestionnaire du flux : au premier tour, il est vide.
- L'ordre des relais par sujet : `tokens` et `agent` ont des fils distincts, mais un jeton qui
  passerait avant `RunStarted` est accepté, puisque la bulle en cours existe déjà (posée par
  `send` avant l'appel).

## La cause (prouvée)

Le rang manquant manque **dès la réception** dans la page (relevé avant tout filtre) : ce n'est pas la page. Aucun `EventsMissed` : ce n'est pas un débordement du bus. C'est `Flux::publier` (ci-dessus, dans l'État). Ce n'était pas toujours le premier jeton : 12, 8, 6, 0 dans les rouges.

## La piste d'avant (everything declarative)

Le flux relaie chaque sujet du bus par **son propre fil** (`IdeFlux::relayer`, un fil par sujet de
`SUJETS`), et l'id du flux est donné à la publication : un `Token` (sujet `tokens`) peut passer
avant le `RunStarted` (sujet `agent`) du même tour. Si la page tient encore un run dans `runRef`
(celui d'avant, ou celui qu'une relecture a posé), le filtre `ev.run !== runRef.current` jette ce
jeton ; sous charge, les fils se décalent davantage. Pour le prouver : `Token` porte `seq` (son rang
dans le tour) ; noter dans la page le premier `seq` reçu par run, un `seq` > 0 en premier le dit.
Correctifs possibles : ne jamais filtrer un `Token` d'un run inconnu quand aucun tour n'est en vol
côté page, ou relayer `tokens` et `agent` par un même fil.

## Ce qu'il faut pour le fermer

Trouver par où le premier jeton se perd : la page (une mise à jour d'état qui l'écrase ?), le
flux (rejeu, `Last-Event-ID`) ou l'hôte. Ensuite, un témoin qui compte les jetons reçus par la
page pendant le flux, contre ceux que le faux modèle a émis, sous charge.
