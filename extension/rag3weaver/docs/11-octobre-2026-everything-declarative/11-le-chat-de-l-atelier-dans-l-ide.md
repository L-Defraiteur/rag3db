# Le chat de l'atelier dans l'IDE : l'agent qui édite les gabarits, vu et arrêté depuis la page

25 octobre 2026, everything declarative. C'est la marche 3 de la page 05 : « Le chat de l'atelier
(`declare`, `refused`, `reloaded`) entre dans l'IDE comme une vue ». Et la marche 2 de la vision
(`visions/00-vision-generale.md`, « l'atelier comme IDE des gabarits ») : un agent dont l'objet est
le gabarit, un composant React compris. Son témoin tourne sur Gemini, pas sur un modèle local
(décision de c7, 19 octobre).

## Ce qui existe

- **Le chat de l'atelier** (`src/atelier.rs`, `serve --atelier`, un backend à la fois).
  `POST /_atelier/chat` lance un tour de `ChatSession::run` (`src/chat.rs`). Ses outils sont ceux
  du manifeste plus `DeclarationTools` (`list_declarations`, `read_declaration`, `declare`). Il
  sauve sa session dans `<backend>/.atelier/session.json`. Ses événements sortent par un SSE fait
  main (`/_atelier/events`) : `token`, `tool_start`, `answer`, `declared`, `refused`, `reloaded`.
- **La boucle d'agent publie sur un bus privé** (`EventBus::new(64)` propre au run, `chat.rs`).
  Personne d'autre n'y est abonné. Le sujet `agent` de l'IDE ne reçoit donc rien d'un chat, et
  `tokens` ne reçoit que les jetons des graphes (`TokenEvents`). La page de l'IDE ne s'abonne
  d'ailleurs ni à l'un ni à l'autre.
- **L'IDE servi par l'hôte** n'a ni route de chat ni session d'agent. Les propositions (page 08)
  et les tâches (pages 09 et 10) sont pilotées par les outils de l'hôte, sans modèle.
- **Le modèle.**
  - Il vient de `models.llm` du manifeste (`model_source::connect_llm`, Vertex par
    `gcp_auth::TokenSource::from_env()`).
  - À défaut, `regime::modele_agentique_nomme` prend `google/gemini-3.8-flash`
    (`MODELE_VERTEX_PAR_DEFAUT`), remplaçable par `VERTEX_MODEL`, avec `GOOGLE_CLOUD_PROJECT`.
  - Les identifiants sont un compte de service, désigné par `GOOGLE_APPLICATION_CREDENTIALS`. Les
    témoins existants le font pointer sur `.vault/vertex-sa.json` de l'arbre principal. Aucun
    code ne lit `.vault` lui-même, et ce chantier ne le lira pas non plus.
- **Le compteur.** `Agent::meter_call` émet `Consumed` sous le slug `llm.gemini-3.8-flash`, mais
  `with_meter` n'est pas branché dans le chat.

## Décisions proposées

1. **Le chat est un service de l'hôte, par backend.** `POST /ide/chat {backend, message}` lance un
   tour, et `GET /ide/chat?backend` rend la session. La session vit dans le dossier d'état de
   l'hôte, par backend, et pas dans le dossier du backend : un backend de tâche (page 09) a la
   sienne, et rien n'atterrit dans un arbre suivi. Un tour à la fois par backend ; un second
   message pendant un tour est refusé en le disant.
2. **Les outils de l'agent sont ceux que la déclaration donne pour ce backend.**
   - En mode `approbation` (le défaut), l'agent **propose** (`proposer`, page 08) : ses
     changements arrivent dans la vue des propositions, et c'est l'humain qui applique.
   - Dans une tâche (mode `auto`, page 09), il déclare sur la branche de la tâche.
   - Sur un arbre inscrit (page 10), l'édition de code passe par la même garde : `task_writes`
     le renvoie vers une tâche.

   Rien de neuf à inventer : ce sont les règles déjà posées, appliquées à un agent de plus.
3. **La boucle publie sur le bus du backend**, plus sur un bus privé.
   - Sur `agent` : le run de l'agent (`RunStarted`, kind `agent`, cible `{chat: <backend>}`),
     chaque appel de modèle, chaque appel d'outil, `Consumed`.
   - Sur `tokens` : les jetons du tour.

   Le flux de l'IDE les relaie déjà ; la page s'y abonne. Les runs de graphes que l'agent
   déclenche par ses outils ont déjà leur publication sur `dataflow` (le DAG vivant s'allume
   pendant que l'agent travaille).
4. **Le tour s'inscrit dans les runs en vol** (C4). Arrêter le chat, c'est `stop_run` sur son
   run, par le même chemin que tout run, et le bouton Arrêter de la page n'en est qu'un client.
   L'`AtomicBool` d'annulation du chat, aujourd'hui jamais levé, est relié à la poignée.
5. **Le compteur est branché** (`with_meter`), sous le slug du modèle, et la page montre la
   consommation du tour.
6. **Le modèle.** Celui du manifeste, sinon `gemini-3.8-flash` par `modele_agentique_nomme`.
   Sans identifiants, le chat est éteint, et la vue le dit avec la raison, comme
   `/_atelier/state` (`chat: {on: false, why}`).
7. **La vue** `/ide/code/<id>/chat`.
   - Le fil de la session : messages, jetons en cours, appels d'outil repliés, résultat.
   - Un tour qui propose montre un lien vers sa proposition. Un tour qui déclare montre `declared`
     ou `refused`, avec la raison.
   - Le bouton Arrêter.
   - La consommation du tour.

## Le témoin

`l_agent_de_l_ide_propose_un_gabarit_sur_gemini`, dans le navigateur, sur un vrai modèle.

- **Le banc.** Le compteur à graphe de clic des témoins d'aperçu (`n + 1`), en mode
  `approbation`. La demande : « Fais compter le compteur par cinq. »
- **Ce qui est jugé, sur le résultat et pas sur le texte du modèle.**
  - Le tour finit sans erreur.
  - Une proposition paraît dans la vue des propositions, par le flux. Elle touche
    `components/compteur.clic.mmd` et se charge (pas de `refus`).
  - Son aperçu, sans appliquer, compte par cinq.
  - Rien n'est écrit sur le disque avant Appliquer.
  - Le flux a porté au moins un appel d'outil de l'agent sur `agent` et des jetons sur `tokens`.
  - La consommation sous `llm.gemini-3.8-flash` est non nulle.
- **Un second tour, arrêté** depuis la page pendant qu'il tourne : il finit `requested`,
  `cancelled_by` vaut `ide`, et rien n'est proposé.
- **Sans identifiants**, le témoin se saute nommément, et rougit sous
  `RAG3WEAVER_EXIGER_GEMINI=1`. On ne remplace jamais Gemini par un modèle local pour le juger.

Un témoin de bout en bout sur un faux modèle (`CallbackLlm`) couvre la tuyauterie sans réseau :
les événements sur le bus du backend, la session, la garde du tour unique et l'arrêt.

## Les lots

- **K1** : le service de chat de l'hôte (route, session par backend, tour unique). L'agent publie
  sur le bus du backend, le tour est inscrit dans les runs en vol, le compteur est branché. Témoin
  sur faux modèle.
- **K2** : la vue du chat dans l'IDE (fil, jetons, outils, propositions liées, Arrêter,
  consommation). Témoin navigateur sur faux modèle.
- **K3** : le témoin sur Gemini (la proposition qui compte par cinq, l'arrêt d'un tour).

## Tranché par c7 (25 octobre)

- **La session vit dans le dossier d'état de l'hôte, par backend**, comme le registre des tâches et
  les runs : c'est de l'état de l'hôte, pas du contenu déclaré du projet. `.atelier/` reste au
  projet (`serve --atelier`).
- **Elle suit la règle de survie du registre des tâches.** Elle est relue au démarrage. Une session
  dont le backend a disparu est orpheline : elle est dite, et jamais rouverte en silence.
- **Un seul message système** : le même agent, et le verbe vient du mode de la déclaration.
  - En approbation, c'est « proposer ».
  - Dans une tâche, c'est « déclarer sur ta branche ».

  Deux textes divergeraient à la première retouche.
