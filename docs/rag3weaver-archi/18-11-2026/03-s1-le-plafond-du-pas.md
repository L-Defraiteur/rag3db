# S1 : le plafond du pas — le 18 novembre 2026

Branche `plafond-du-pas`, worktree `~/.cache/rag3weaver-build/wt-delegate`. Ticket :
`docs/rag3weaver-archi/tickets/2026-11-16-le-plafond-d-appels-d-un-pas.md`.

## Ce que montre la mesure

J'ai relevé les quatorze pas de la campagne D6 du 16 novembre dans le journal des runs
(`~/.local/share/rag3weaver/runs/e9c92e8594cc/agent-*.json.zst`, événements `LlmCall`) :

- **Il y a deux plafonds à 12, pas un.** `STEP_BUDGET = 12` borne les appels d'outils (le ticket).
  Il y a aussi `max_iterations` du tour de chat : l'hôte compose sa `ChatConfig` sans ce champ
  (`daemon_host/chat.rs`), donc avec le défaut de `chat.rs`, 12 appels au modèle. Sept pas sur
  quatorze s'arrêtent à **12 appels au modèle**. Relever `STEP_BUDGET` seul ne changerait rien.
- **Le contexte d'un pas** monte de 5,8 k jetons (l'invite et la frontière) à 14 à 21 k au 12e
  appel.
- **Le cumul des jetons d'entrée** d'un pas de 12 appels fait de 125 à 153 k. Le chiffre du ticket
  (« 60 000 jetons d'entrée cumulés ») était donc faux : il couperait les pas plus tôt
  qu'aujourd'hui. Un cumul mesure un coût, pas l'épuisement d'un pas, puisque chaque appel renvoie
  tout le contexte.

## Conception, en quatre lignes

1. **Ce qui est borné, c'est le contexte du pas.** `teammate.step_context_tokens` (défaut 64 000,
   environ trois fois le pic mesuré) compare les jetons du dernier appel au modèle (entrée et sortie,
   lus sur `generation_end`) au budget. Au-delà, la porte refuse les outils de travail en disant
   « le contexte du pas est plein : déclare step_done (fait, reste) ». `step_done` et les outils de
   la boîte passent toujours.
2. **Deux bornes de sûreté, réglables, très au-dessus** :
   - `teammate.max_tool_calls_per_step` (défaut 60) remplace `STEP_BUDGET` ;
   - `teammate.max_model_calls_per_step` (défaut 40, sous le plafond de 100 de `ChatConfig`) est
     passé en `max_iterations` au tour d'un pas.

   Le refus nomme la borne atteinte.
3. **Ce qui ne bouge pas** : `MAX_STALLED`, la garde B, la priorité des accords, le tour de chat
   hors d'un pas. Les clés sont neuves dans `TeammatePolicy` (`src/backend.rs`), en anglais, avec
   les valeurs par défaut ci-dessus.
4. **La preuve** :
   - rouge d'abord sur le faux modèle (`teammate_tests.rs`, `CallbackLlm`) : un pas fait 15
     lectures, puis un `edit_file`, puis `step_done`, et finit en un pas avec le fichier écrit.
     Aujourd'hui le 13e appel est refusé ;
   - un second témoin rend un `generation_end` au-dessus du budget, et la porte refuse avec le
     message du contexte ;
   - puis la passe complète et e2e_equipier sous Vertex.

Le pas suivant qui ne voit pas les retours d'outils (S2) reste un ticket à part : avec un pas plus
long, il pèse moins, mais il ne disparaît pas.
