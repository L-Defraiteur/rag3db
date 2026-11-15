# tty-14 : `/model`, le modèle de langage d'une tâche

Session tty, 15 novembre 2026. c7 a ouvert le lot une fois `models.llm` en liste posé sur master (`f9e894f36`, ED) : `/model` liste les modèles déclarés, `/model <nom>` change celui de la tâche courante. Le vocabulaire est celui du code d'aujourd'hui (`provider`, `protocol`), et l'écran montre surtout `name` et `model`.

## Côté hôte, ce qui manquait

Le chat d'un backend prenait toujours le modèle par défaut de son manifeste (`models.get(&Capability::Llm)`), et aucune tâche ne retenait de modèle à elle. Le lot ajoute, sans toucher à `ModelSource`, `Modeles` ni `ChatConfig`, et en passant par `Modeles::modeles_de_langage`, `Modeles::nom` et `Modeles::modele_de_langage` (la surface prévue, relue par ED) :

- **`Tache.modele`** : le `name` choisi, gardé dans le registre des tâches (`registre.json`), absent tant que la tâche suit le défaut ;
- **`GET /ide/teammate/models?backend=<id>&task=<nom>`** : `{models: [{name, model, provider, protocol, default, current}], current, current_unknown, default}`. Le défaut est celui que garantit la lecture de `Modeles` (`.get(&Llm)`) ;
- **`POST /ide/teammate/model {backend, task, name}`** : la tâche emploie ce modèle déclaré. Un nom inconnu est refusé par le texte de `modele_de_langage`, qui nomme la liste ;
- **le chat de la tâche suit le choix**, aux deux endroits où l'hôte prend le modèle (`chat_state`, `run_chat_turn`) : le modèle de ce nom, connecté par sa déclaration (`connect_llm`), sinon le `ChatModel` comme avant ;
- **un choix qui ne désigne plus rien** (le manifeste renommé depuis), les deux ajouts d'ED : la liste le rend dans `current` avec `current_unknown: true`, et le tour du chat est refusé avec « … ; choisissez un modèle par /model », jamais le défaut en silence.

## Côté terminal

- `/model` seul : les modèles de la tâche, `name — model (provider, protocol)`, celui de la tâche marqué `▸ … · celui de la tâche`, le défaut `· par défaut`. Un choix devenu inconnu se dit « ▸ ancien (inconnu) — le choix de la tâche ne désigne plus aucun modèle déclaré ».
- `/model <nom>` : « la tâche emploie maintenant le modèle « nom » (model) », ou le refus de l'hôte tel quel.
- Tab complète `/model ` par les noms des modèles de la tâche.

## Les témoins

- **Hôte** (`src/daemon_host/modele_de_tache.rs`, sous `code`, `rag3db-native` et `openai-llm`). Deux modèles compatibles sont déclarés dans le manifeste d'une tâche ouverte, que l'hôte relit à la réouverture. Le témoin vérifie la liste, avec le défaut et le courant marqués, puis le choix gardé dans le registre, puis la jauge de `chat_state`, qui suit le modèle choisi (fenêtres de 1000 contre 2000 jetons), le nom inconnu refusé en nommant la liste, et la tâche absente refusée. Il vérifie enfin le choix devenu inconnu et le refus du tour qui dit quoi faire.
- **Terminal** : la lecture de `/model`, sa complétion, la mise en forme de la liste et du choix inconnu.

Ce qui a été vu rouge, et les faux pas du chemin :
- le premier rouge de l'hôte tombait sur l'ouverture du backend, pas sur les méthodes : un manifeste qui déclare `models.llm` refuse de s'ouvrir sans `openai-llm`. La ligne avait été lue, pas le message ;
- puis le jeu d'essai déclarait un fournisseur par défaut sans démon, puis il écrivait les modèles dans le manifeste du principal, que la tâche ne lit pas ; un diagnostic l'a montré (la tâche n'avait que `Sparse`). Le témoin déclare maintenant ses modèles dans le manifeste de la tâche et la rouvre ;
- un push rejeté par GitHub (« remote rejected (failed) ») a fait rejouer luciepc sur un ancien jeu d'essai. La branche distante est vérifiée avant chaque passe depuis ;
- les deux méthodes de l'hôte ont été vues rouges contre leur version faussée dans le worktree de luciepc (sans commit), le choix inconnu sur le commit qui ne portait que son test. Puis tout est vert.

La lib entière de rag3weaver sous `rag3db-native,code,daemon,openai-llm` : 2426 tests verts, 72 ignorés, aucun rouge (sur `c2fa35406`) ; `check_features` 8/8. Le crate tty passe à 97 tests avec la vue, et en garde 19 sans elle.
