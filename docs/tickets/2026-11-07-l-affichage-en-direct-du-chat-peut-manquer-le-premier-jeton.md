# L'affichage en direct du chat peut manquer le premier jeton du premier tour, sous charge

- **État** : ouvert (7 novembre 2026), relevé par hooks-essai dans le lot « le chat tient
  l'ordre » ; la relecture de fin de tour fait foi (6d3504d56), la cause n'est pas trouvée.
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

## Ce qu'il faut pour le fermer

Trouver par où le premier jeton se perd : la page (une mise à jour d'état qui l'écrase ?), le
flux (rejeu, `Last-Event-ID`) ou l'hôte. Ensuite, un témoin qui compte les jetons reçus par la
page pendant le flux, contre ceux que le faux modèle a émis, sous charge.
