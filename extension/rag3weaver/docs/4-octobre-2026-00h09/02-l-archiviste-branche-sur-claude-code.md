# L'archiviste branché sur Claude Code — étude

4 octobre 2026. Question de Lucie : « avec les mods de Claude Code, on pourrait
potentiellement avoir l'archiviste même pour Claude Code ? Ça permet quoi ? »

Ce document complète la vision (`01-…`, §9). C'est une **étude**, pas une
décision : rien n'est codé.

**Réserve.** Ce qui suit est un relevé de la documentation de Claude Code fait
le 4 octobre 2026, non vérifié point par point contre un essai. Deux points
sont à confirmer avant de construire dessus ; ils sont marqués.

## 1. Ce que Claude Code laisse brancher

| Mécanisme | Ce que c'est | Ce qu'il apporte à un archiviste |
|---|---|---|
| **Crochets** (*hooks*) | des commandes lancées à des moments précis de la session | le fil des événements, et la porte d'injection |
| **Serveur MCP** | des outils exposés à l'agent | `remember`, `recall`, `add_ref`, `create_ref_type` |
| **Plugin** | un paquet : crochets, serveur MCP, commandes, sous-agents | l'installation en un geste |
| **Sous-agent** | un agent au contexte séparé | résumer sans encombrer l'agent qui travaille |
| **SDK d'agent** | les mêmes crochets, par programme | un harnais à soi, si l'on veut plus de maîtrise |

## 2. Les crochets, le cœur de l'affaire

Moments auxquels un crochet se déclenche, pour ce qui nous sert :

| Moment | Ce qu'on y ferait |
|---|---|
| début de session (démarrage, reprise, après compression) | injecter le sommaire du classeur |
| message de l'utilisateur | la porte à deux temps : chercher ? injecter ? |
| avant un appel d'outil | rien, sauf une garde |
| après un appel d'outil | injecter les fiches accrochées à ce que l'outil vient de toucher |
| avant la compression du contexte | écrire le L1 de la tranche qui va disparaître |
| après la compression | noter qu'elle a eu lieu |
| arrêt du modèle, fin de session | la passe de rattrapage, les L2 du jour |

Chaque crochet reçoit l'identifiant de la session, le dossier de travail, et
**le chemin du fichier de transcription** de la session : un fichier d'une
ligne par événement, avec les messages, les appels d'outils et leurs résultats.
Un processus extérieur peut donc lire toute la session.

Certains crochets peuvent **ajouter du contexte** à la conversation (début de
session, message de l'utilisateur, après un outil) ; celui d'avant-outil peut
refuser ou modifier un appel.

## 3. Nos cinq gestes

| Geste | Possible ? | Par quoi |
|---|---|---|
| lire le fil de la session | oui | la transcription donnée à chaque crochet |
| savoir que le contexte se compresse | oui | les crochets avant et après compression |
| savoir que le contexte atteint 30 % | partiellement | aucun crochet ne donne le remplissage ; à calculer depuis la transcription |
| écrire les résumés dans notre mémoire | oui | un crochet qui appelle notre binaire ou notre serveur |
| injecter des fiches avec le résultat d'un outil | oui — **à confirmer** | le crochet d'après-outil ajoute du contexte |
| injecter un sommaire au démarrage | oui | le crochet de début de session, y compris après une compression |

**À confirmer par un essai** : ce que le crochet d'après-outil injecte
exactement (une note attachée au résultat, ou du contexte général) ; et ce que
reçoit le crochet d'après compression (le résumé produit, ou seulement le fait
qu'elle a eu lieu).

## 4. Les limites

- **Le format de la transcription est interne** à Claude Code et peut changer
  d'une version à l'autre : l'adaptateur qui la lit doit être petit, testé, et
  tolérant.
- **Le crochet de fin de session a très peu de temps** (de l'ordre d'une
  seconde et demie par défaut) : la passe finale tourne en arrière-plan, pas
  dans le crochet.
- **On ajoute, on ne retire pas** : pas d'accès aux réflexions internes du
  modèle, pas de modification du prompt système, pas de réécriture de
  l'historique.
- **Chaque injection coûte du contexte** — le souci de Lucie (« pas injecter
  H24 ce que l'autre a déjà en tête ») vaut ici tel quel : la porte à deux
  temps et le registre de ce qui a été montré sont nécessaires, pas
  facultatifs.
- **Un crochet s'exécute avec les droits de la personne** : un archiviste
  installé chez quelqu'un lit toutes ses sessions. À dire clairement, et à
  garder local.

## 5. Ce que cela veut dire pour notre conception

La forme retenue pour l'archiviste — **un processus à part, qui lit un journal
et n'injecte que par une porte étroite** — est exactement ce que les crochets
offrent. Rien n'est à redessiner : il faut deux adaptateurs pour un même
archiviste.

| | Notre chat | Claude Code |
|---|---|---|
| le fil lu | `journal/<session>.jsonl` | la transcription de la session |
| le remplissage du contexte | le cumul de jetons de chaque fin de tour | à calculer ; la compression se sait par son crochet |
| la porte d'injection | le crochet après outil (livré) | les crochets de début de session, de message et d'après-outil |
| les outils de mémoire | les gabarits du backend | un serveur MCP qui expose les mêmes verbes |
| l'installation | le manifeste du backend | un plugin |

Conséquence de produit : la mémoire longue pourrait servir des gens qui
utilisent déjà Claude Code, sans qu'ils changent d'outil. C'est une piste à
part entière, à côté des deux produits code.

## 6. Par où commencer, si on le fait

1. La moitié « noter », hors ligne : un binaire qui lit une transcription et
   écrit ses L1 dans une mémoire — aucun crochet d'injection, aucun risque
   pour la session. Il dit tout de suite si le format se lit et ce que valent
   les résumés.
2. Le crochet d'avant-compression et celui de fin de session, qui lancent ce
   binaire.
3. Le serveur MCP (`remember`, `recall`) : la personne et l'agent tirent sur
   la mémoire à la demande.
4. L'injection, en dernier, éteinte par défaut : le sommaire au démarrage
   d'abord, les fiches après un outil ensuite.

## 7. Sources

Documentation de Claude Code consultée le 4 octobre 2026 : les crochets
(`code.claude.com/docs/en/hooks`, `…/hooks-guide`), les sessions
(`…/sessions`), la fenêtre de contexte (`…/context-window`), MCP (`…/mcp`).
