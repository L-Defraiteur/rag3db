# Rapport de la session d'orchestration — 11 octobre 2026, midi

Session `rag3db-c7`, orchestration. Suite du rapport de nuit
(`../../10-octobre-2026-18h20/orchestration/01-rapport-de-session.md`, §6 à
§19) et des décisions du matin (`01-les-decisions-du-matin.md`). Écrit à la
demande de Lucie parce que le contexte de la session est lourd : ce document
et `03-knowledge-dump.md` à côté suffisent pour reprendre l'orchestration
depuis une session vierge.

## 1. Où en est master à midi

Tête : `442791fde` (le squelette de la page de l'enveloppe d'un outil
d'écriture, mémoire + arbre principal). Depuis le matin, sur master :

| Lot | Session | Hash | Ce que ça change |
|---|---|---|---|
| T0, condition 3 première marche | cœur | `82a5f4851` | refus au COMMIT d'une transaction en échec (écart Neo4j, pas de ROLLBACK muet) |
| Marche 0 du démon | arbre principal | `75de6618f` | un serveur qui se ferme proprement et sur SIGTERM ; deux tickets passés en « partiel » (`970a5c1a2`) |
| Stèle du produit | arbre principal | `9bd5f1892` | P1–P4, vingt tickets rangés dessous ; index des tickets en deux tables d'ouverts (moteur / produit) |
| COPY de blobs sous petit tampon | hotfixs + A | `720b45ff9`, `bdad46420` | quatre variantes rouges, le forçage n'est pas la cause, forme moteur seule et reportée |
| Vérité sur `atomic` | mémoire | `2d0687a71` | une instruction : `true` ; plusieurs liens seuls sous dialecte transactionnel : enveloppe begin/commit/rollback, `true` ; fermeture avec création/mise à jour/dérivation : `false` + `atomic_reason` ; témoin rouge par injection de faute (`RAG3WEAVER_TEST_LINK_FAULT_RELATION` + `…_GROUP`, jamais l'une sans l'autre) |
| Les e2e en un binaire | recherche | `4cd52e456` … `61bd75055` | **target propre 23 → 4,4 Go**, 97 binaires → 1 (358 Mo), rebâti 43 → 25 s ; plafond 20 Go refusé par `run_e2e.sh` ; staticlib hors du crate-type (seul le bâti WASM la demande) ; 14 suites qui se relançaient par `current_exe` avec un nom court corrigées (contre-épreuve rouge : 13 échecs) ; résumé apparié par nom, pas par rang |
| Enveloppe d'un outil d'écriture | mémoire + A | `442791fde` | page commune `11-octobre-2026-arbre-principal/02-l-enveloppe-d-un-outil-d-ecriture.md`, squelette ; pas de code avant lecture |

Le reste du matin est dans `01-les-decisions-du-matin.md` §4 (lien générique,
receveurs non typés, marche 0, stèle du produit, agents et DAG).

## 2. La ventilation des 4,4 Go (réponse à « c'est purement le debug ? »)

Non. Le débogage des dépendances est déjà coupé, et passer notre crate en
`line-tables-only` ne gagne rien (mesuré par la recherche). Les 4,4 Go sont :
3,2 Go de code compilé de 591 crates, 1,1 Go d'exécutables (le binaire e2e et
les trois binaires de service que les tests lancent), 0,3 Go de proc-macros et
d'objets C/C++. Ce qui reste possible est déconseillé : retirer les symboles
(−0,35 Go, traces sans nom de fonction) ; sortir les trois binaires de service
dans leur paquet (−0,67 Go, structurel). Page
`11-octobre-2026-recherche/03-les-e2e-en-un-binaire.md`.

**Conséquence, décidée par Lucie ce matin et déclenchée maintenant** : la règle
du target bascule. Le target cargo (ou le `build/` C++) d'une session est
**gardé entre ses lots, effacé en fin de session**, plafond 20 Go ; les
worktrees restent effacés après chaque fusion. Diffusée à toutes les sessions
à midi.

Reste ouvert, en lot à part avec les propriétaires : `common::dossier_de_suite()`
pour les sept suites qui veulent un vrai disque, et le `checkpoint_dir` de test
(955 Mo dans le vrai cache). Proposé au banc (au repos) d'abord, au cœur en
lecture.

## 3. Les agents qui se parlent : ce qui existe, ce qui manque

Lucie, ce matin : « je veux qu'ils puissent se parler ; on avait prévu des
trucs pour multi-agents qui se parlent, c'est passé où ? ». Relevé fait par un
agent de lecture sur les visions, les docs et le code. Rien n'a disparu : la
plomberie date de la nuit du 26 août et elle est là ; ce qui n'a jamais été
branché, ce sont les outils qui permettraient à un agent d'écrire à un autre
ou d'en lancer un.

**Les visions** : `visions/00-vision-generale.md` l. 235, marche 6 « À
plusieurs » (écritures parallèles, présence entre agents, démo « la page que
tout le monde édite ») ; `2026-10-04-14h58-fiches-de-contexte…` §2 le
curseur, « un curseur visible des autres est un verrou d'intention, qui annonce
sans bloquer », registre « à faire » ; `2026-10-04-16h24-montrer-nos-produits.md`
l. 648-676, « une équipe de demain qui parle à un site déjà déployé », règle
quand deux demandes se contredisent « à débattre » ;
`2026-10-03-20h37-le-produit-code.md` §6 un arbre de travail par agent ;
`2026-10-09-13h36-plusieurs-produits…` §6 le magicien du chaos qui « te donne
le lien de l'agent dédié » ; `2026-10-04-00h09-l-archiviste…` l. 20 le
sous-agent au contexte séparé.

**Les docs de session** : `docs/26-aout-2026-20h29/12-conversations-a-plusieurs.md`
(le modèle : Conversation, Participants, Posture par couple, verbes
`pause_dialogue` / `confirm_pause` / `leave`, détection d'attente circulaire ;
pauses et blocages faits, `leave` jamais écrit) ;
`docs/27-aout-2026-13h01/09-le-terminal-a-plusieurs.md` (« Alma architecte,
Zed reviewer » ; première expérience à trois agents sans conclusion ; §7 ce qui
manque : fil à plus de deux et diffusion faits depuis, rôle attaché à une
identité et gabarits `builtin/reviewer`, `builtin/architect` restés promis) ;
`docs/vision_roadmap_09_2026/07-evenements-runs-et-boucles.md` §5 (tout fait
sauf `interrupt` et la boîte durable `MessageSourceNode`) ;
`docs/vision_roadmap_09_2026/09-trois-roles-et-une-seule-main.md` (agent de
code, agent de contexte, agent de design qui joue l'utilisateur, outil
`agentCanAnswer`, souffleur : notés « manque », jamais confiés) ;
`docs/1-octobre-2026-22h47/03-releve-des-visions-02-a-08.md` l. 168 « deux
agents qui conversent par leurs boîtes : fait ».

**Le code** (`extension/rag3weaver/`) : la boîte d'un run est le sujet
`run.<id>.inbox` (`src/events.rs:418`) ; `send_message` (`:621`), `send_in`
dans un fil nommé (`:630`), `broadcast` (`:653`) ; `Agent::with_inbox`
(`src/agent.rs:891`) et `read_inbox` (`:1062`) transforment les messages reçus
en tours `user` ; `pause_dialogue` / `confirm_pause` déclarés `:588-590`,
traités `:896`, notification au pair `:955` ; postures et blocages
`src/postures.rs:41` ; bloc d'état `refresh_waiting_block` (`:1222`) ;
entités Conversation / Participant / `PARTICIPATES_IN`
(`src/dataflow/trace_nodes.rs:55-75`), identifiant de fil dérivé de la paire
(`:222`) ou nommé (`:462`) ; `SendMessageNode` (`:1004`) avec destinataire et
contenu **fixés dans la configuration** (`:1051`) ; un réacteur réveille un
agent sur événement (`src/dataflow/reactor.rs`). Tests en `#[ignore]` :
`tests/e2e_agent_loop.rs:771` (deux agents par leurs boîtes, mais c'est le
**test** qui relaie la réponse, `:802`) et
`tests/e2e_conversation_a_plusieurs.rs:266` (trois agents, un fil, vrai
modèle). Le chat (`src/chat.rs:228-230`) ne fait tourner qu'un agent, sa boîte
ne reçoit que les résultats des commandes en fond ; `RunStarted` est émis avec
`parent: None` (`src/agent.rs:1145`). Aucun registre de curseurs, aucun verrou
d'intention, rien du magicien.

**Ce qui manque** : aucun outil exposé au modèle pour écrire à un pair ou dans
un fil (les seuls messages qu'un agent émet seul sont les notifications de
pause) ; aucun nœud, outil ni lien parent pour qu'un agent en lance un autre ;
restent promis la boîte durable, `interrupt`, les rôles attachés à une
identité, les gabarits `builtin/*`, la présence (curseurs) et `leave`.

**La contrainte de Lucie (midi)** : « un agent est un DAG ; un DAG est
potentiellement un outil quand on a besoin qu'il le soit, dont les agents ; ce
que ça veut dire n'est pas forcément d'attendre son retour : peut-être juste le
lancer en fond avec un message initial et la référence de qui je suis, qu'il
puisse me parler ». Forme à proposer (non encore cadrée, non confiée) :

1. **Tout graphe déclaré est un outil potentiel**, les agents compris ; ce
   n'est pas un type de plus, c'est une projection du manifeste (nom, entrées,
   sorties) vers une déclaration d'outil, comme la commande en fond l'est déjà.
2. **Deux modes d'appel du même outil** : attendre (le retour est la valeur) ou
   lancer en fond (le retour est la référence du run, le message initial part
   dans sa boîte avec la référence de l'appelant, `RunStarted.parent` renseigné).
3. **Deux outils génériques exposés au modèle** : `send` (à un run ou dans un
   fil nommé ; la plomberie `send_message` / `send_in` / `broadcast` existe) et
   `inbox` n'a pas besoin d'outil (les messages reçus deviennent des tours
   `user`, c'est fait). Le destinataire choisi par le modèle, plus fixé dans la
   configuration : c'est la différence avec `SendMessageNode`.
4. **Le chat et le serveur MCP branchent la boîte** aux messages des pairs, pas
   seulement aux résultats des commandes en fond.
5. **Bornes et politique** au manifeste de l'agent appelé (qui peut le lancer,
   combien à la fois, durée), et la trace en base (Conversation, Participant,
   parent) : ce qui existe déjà dans `trace_nodes.rs`.

Qui et quand : à décider par Lucie (maintenant ou après la marche 1 de C ;
sessions candidates : arbre principal qui tient le démon et le chat, ou la
session « everything declarative » qui tient les nœuds scriptés et `serve`).

## 4. Ce que chaque session tient à midi

- **cœur C++** (`coeur c++`, luciepc) : A3′ reprise « pierre tombale », C7
  après arrêt brutal ; A9 mesure du mode puis bascule du défaut sur son coût
  dit. Banc / cas temporels en `poste mesure` sur luciepc.
- **hotfixs** (`coeur c++/hotfixs/tickets`) : A7 + A8 ; COPY de blobs borné
  en octets reporté après condition 3 (page écrite).
- **arbre principal** (`rag3db-73`) : marche 1 de C (fil client mince ↔ démon,
  **la session d'abord** : le client s'enregistre, porte sa cellule et ses
  clés) ; sa part de la page de l'enveloppe (transaction par paquet, ce qui
  vit hors de la base, « un outil de backend peut-il empoisonner son catalogue
  sous C ? ») ; relecture faite du binaire e2e.
- **embarquements / F** (`rag3db-6f`) : fusion de l'échappement systématique
  des identifiants + trois classes de refus (dont « champ intégré » pour
  `uuid`…) + section IR/témoin ; Select PG pour la mémoire (marche 1 bis
  l'attend).
- **mémoire** (`rag3db-96`) : page de l'enveloppe (sa part : contrat vu de
  l'outil et de l'agent, recensement des outils d'écriture des gabarits livrés,
  témoins) ; marche 1 bis (`Catalog::link` refuse un bout absent) attend F.
- **codeparsers** (`rag3db-ab`) : T5, receveurs restants, l'ordre des lots au
  §3 de la stèle du produit ; sa part de codeparsers à écrire.
- **recherche** (`rag3db-97`) : **fini** le lot e2e/disque ; en attente d'un
  top pour `dossier_de_suite()` et `checkpoint_dir` avec banc et cœur.
- **banc** (`rag3db-90`) : au repos ; C7 témoin minimal à sa reprise ;
  candidat pour les sept suites à vrai disque.
- **G, paquet npm** (`rag3db-10`) : en pause, quatre archives `0.0.1-alpha.2`
  prêtes sous `~/.cache/rag3weaver-build/paquet-npm-garde/publier/`.
- **proto tout déclaratif** (`everything declarative`) : en pause, six lots sur
  master, recette pour Lucie page `everything-declarative/08` §1.
- **lucied-68** : session de Lucie, pas cadrée par l'orchestration.

## 5. Ce qui attend Lucie

- La forme « un DAG est un outil, lancé en fond avec un message et la
  référence de l'appelant » (§3) : ouvrir maintenant ou après C, et à qui.
- La séance npm (quatre OTP, commandes au §16 du rapport de nuit).
- `claude mcp add` depuis une session vierge ; le conteneur pgvector.
- Le ménage des branches distantes (liste à produire avant).

## 6. Fautes de la session (pour ne pas les refaire)

- Deux hypothèses rendues avant la mesure (« analyze automatique après COPY »,
  « le forçage est la cause ») : réfutées par les sondes, retirées. La preuve
  dans le code d'abord, la mesure confirme, et une hypothèse se dit comme
  telle.
- Un « outil d'ancrage en six lignes » accepté sur parole, retiré après mesure.
- Une règle diffusée (« effacer le target à chaque fusion ») dont la cause
  réelle était ailleurs (moteur statique + empreintes) ; la cause mesurée a
  rendu la règle temporaire. Toujours ventiler avant de légiférer.
