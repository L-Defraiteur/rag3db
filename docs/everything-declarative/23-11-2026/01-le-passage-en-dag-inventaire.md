# Le passage en DAG : l'inventaire du code en dur

23 novembre 2026, session everything declarative. Lucie, vers 17 h 20, relayée par c7 : « une fois qu'on a l'agent de
code fixé pour de bon, on se fait une opération passage en DAG du code au max : tout ce qui peut devenir un DAG purement
et moins de code en dur le devient, dans la boucle de l'agent, donc définissable par mermaid et gabarits aussi, tout,
même le fonctionnement du terminal ». Chantier à moyen terme. **Rien n'est codé maintenant** : cette page prépare le
travail.

Le relevé porte sur `origin/master` du 23 novembre. Il couvre la boucle de l'équipier (`teammate.rs`, `card.rs`,
`juge.rs`), la boucle de l'agent et ce qui l'entoure (`agent.rs`, `chat.rs`, `land.rs`, `gate.rs`, `decider.rs`, les
cartes d'accord) et le terminal (`tty/src`). Les lignes citées sont celles de ce jour.

## La règle de tri

La frontière posée par Lucie le 20 novembre : **la flexibilité va au script, aux graphes et aux gabarits ; les
primitives et la performance restent en Rust.** S'y ajoute une troisième classe, celle des **gardes dures**, qui restent
en Rust quel que soit le reste.

Pour chaque point, la page dit :
- s'il est déclarable **aujourd'hui** ;
- s'il manque une **primitive** du moteur de graphes, et laquelle ;
- si c'est un **chemin chaud** (par jeton, par appel d'outil) ;
- si c'est une **garde dure**.

## Ce qui est déjà déclaré

- **Les ports de la boucle de l'agent** (`agent_ports.rs`) : `user_in`, `peer_in`, `model_out`, `tool_call_out`,
  `tool_result_in`, `answer_out`, `turn_end`. Un sous-graphe branché sur un port réécrit un élément, ou le retient.
- **Le bloc « Previous step » de l'équipier** : un nœud scripté (`teammate.carry_over_node`, en TypeScript ou en
  WebAssembly), avec un repli Rust et l'erreur dite une fois. C'est le modèle à suivre pour le reste.
- **Les règles de la porte** (`rules.json`), le juge des bilans (un agent gabarit, `templates/agents/juge/`), le thème
  et les politiques d'outils du terminal (servies par `/ide/describe`).
- **Les seuils de la boucle**, une vingtaine de champs de manifeste (`TeammatePolicy`, `memory.compact_at`,
  `chat_instructions`, `bilans`…). Les seuils sont déclarés ; **le flux de contrôle qui les lit ne l'est pas**.

## Les primitives qui manquent

Classées par nombre de points qu'elles débloquent.

1. **Une boucle avec condition d'arrêt et un état durable entre les tours** : un compteur, « relancé une fois », un
   curseur. Elle débloque la boucle de l'agent (budgets, coup de pouce final, relance après coupure, arrêt sur deux
   erreurs identiques) et la boucle de l'équipier tout entière.
2. **Une attente d'événement qui bloque, avec une échéance et une annulation** : la fin d'un tour, la décision d'un
   humain, le résultat d'un outil asynchrone, une ligne de boîte. Aujourd'hui, `EventSourceNode` vide la file, il
   n'attend pas.
3. **Une reprise : une fois sur erreur, ou avec un recul croissant et une annulation**. Elle couvre le débordement de
   contexte et ses compactions, les quotas des fournisseurs, les pas échoués. Elle suppose une **erreur typée** : les
   quotas se reconnaissent aujourd'hui au texte de l'erreur.
4. **Une chaîne de gardes, la première vraie l'emporte, sur des prédicats calculés.** `Route` aiguille sur un seul
   champ ; la fin d'un pas d'équipier en enchaîne six.
5. **Des nœuds de boîte** : lire une boîte d'équipier, y ajouter une ligne sans doublon.
6. **Le réacteur avec un vol unique et la reprise au démarrage.** `reactor.rs` sait `each`, `batch` et `debounce`.
7. **Un limiteur de débit**, par exemple les bilans par heure.
8. **Pour le terminal, trois formats** : une machine à états, une table événement → réaction, et un gabarit de vue
   avec son clavier.

Hors primitives, les **jetons** qui traversent un nœud restent compilés : c'est le chemin le plus chaud, une
redessinée par jeton.

## Ce qui reste en Rust, quoi qu'il arrive

- le masquage des secrets (`MaskedTools`) ;
- le registre des dépenses et son plafond (la réservation, le solde, le seuil) ;
- les accords de dossiers (`grant_root`, le plancher des dossiers protégés, les cartes de racine large) ;
- la signature humaine (`by.kind == "human"`) et l'octroi d'une commande (`accorder_ligne`, la porte des commandes) ;
- la fermeture unique d'une carte (sous verrou) ;
- l'identité git de l'atterrissage, qui tient l'adresse du poste hors des dépôts ;
- la politique fermée des types de nœuds ;
- dans le terminal, la réponse à une carte d'accord (jamais par du texte tapé) et l'en-tête `X-Rag3weaver-User`,
  envoyé seulement après une frappe ;
- les bornes vérifiées à chaque appel d'outil (contexte, chemins refusés, nombre d'appels), qui sont un chemin chaud.

Une garde peut devenir un **nœud** dont le corps reste en Rust : le graphe le place, il ne peut pas le contourner.

## L'inventaire

### La boucle de l'équipier (`teammate.rs`, `card.rs`, `juge.rs`)

| Point | Où | Ce qui bloque | Taille |
|---|---|---|---|
| La boucle des pas (lire la boîte, faire un pas, attendre, relancer, finir) | `teammate_loop`, 2273–2810 ; `MAX_STEPS` et `MAX_STALLED` en constantes | primitives 1 et 4 | grande, en dernier |
| Attendre du travail et se réveiller (bus, croissance du fichier, échéance) | 2343–2385 | primitive 2 | moyenne |
| Les refus du fournisseur (quota épuisé, dépassé, panne) | 2491–2541, `refus_du_fournisseur` | primitive 3, erreur typée | moyenne |
| Le pas échoué, réessayé | 2542–2556 | primitive 3 et un compteur | petite |
| **« Est-ce fini ? »** : reste à faire, livrable manquant, tour muet, demande sans trace, juge du contenu, vérification | 2679–2800 | primitive 4 et « relancé une fois » | grande, **la plus utile** |
| Le juge du contenu (manquant, livré, action) | `judge_the_content`, 2742–2766, 2857–2908 | **aucune** : `Decide` puis `Route`, compté au registre | petite, **premier gain** |
| La porte de vérification (`policy.verify`) | 2767–2799 | un compteur de relances (primitive 1) ; `RunCommandNode` existe | petite |
| Le message de frontière (nouvelles, plans, faits, relance, pas précédent) | `frontier_message`, 1555–1606, 2420–2445 | **aucune** : texte pur, un nœud scripté | moyenne |
| Le bloc « Previous step » | 645–740, 900–918 | déjà déclaré ; reste à retirer le doublon Rust | petite |
| La copie du texte d'un pas en compte rendu, la relance sans `step_done` | 2563–2628 | primitive 5 | petite |
| Attendre une question ouverte | 2661–2677 | primitive 2, clé `reply_to` | petite |
| Le plan (`edit_plan`, `step_done`) | 962–1099 | `ReactTransitionNode` existe ; un magasin JSON au lieu d'une entité | moyenne |
| Les bornes d'un pas | `refusal`, `gate`, 1163–1188 | chemin chaud : reste en Rust | petite |
| La porte d'une commande retenue (bac, juge, carte) | `StepTools::after`, 1349–1436 | garde dure ; seul l'aiguillage se déclare | moyenne |
| Les réponses et les accords (`post_to_teammate`) | 1852–2057 | garde dure (`by.kind`, `accorder_ligne`) ; la règle de réveil se déclare | moyenne |
| Lancer, relancer, reprendre au démarrage | `start_teammate`, 2058–2100 ; 1791–1829 | primitive 6 | moyenne |
| La fin d'un équipier (faits, ligne `ended`) | `run_teammate`, 2211–2271 | primitive 5 | petite |
| Le cycle d'une carte (ouvrir, état, fermer une fois) | `card.rs`, 18–77 | garde (verrou) : un nœud dont le corps reste en Rust | petite |
| La file des bilans et leur juge | `juge.rs`, 617–669, 979–996 | réacteur sur la fin d'un run, primitive 7 ; le plafond horaire est une garde de dépense | moyenne |

### La boucle de l'agent et ce qui l'entoure (`agent.rs`, `chat.rs`, `land.rs`, `gate.rs`, `decider.rs`)

| Point | Où | Ce qui bloque | Taille |
|---|---|---|---|
| La boucle d'un run (un appel, ses outils, recommencer) | `Agent::run_inner`, `agent.rs` 1728–2298 | primitive 1 ; les jetons restent compilés | grande |
| Les budgets (itérations, jetons, `user_in` retenu) | 1733–1745 | primitive 1 ; `token_budget` reste un plancher Rust | petite |
| Le coup de pouce du dernier appel (`FINAL_NUDGE`, outils retirés) | 1747–1790 | primitive 1 (savoir quel tour) | petite |
| Le débordement de contexte, réessayé une fois | 1797–1837 | primitive 3, session durable | moyenne |
| **Les appels d'outils écrits dans le texte** (`recover_tool_calls`) | 1850–1885 | **aucune** : un nœud sur `model_out` | petite |
| La relance après une réponse coupée, les relances de complétion | 2030–2053 | primitive 1 ; le nombre de relances est en dur | petite |
| L'aiguillage d'un appel d'outil (pause, envoi, asynchrone, direct) | 2135–2185 | primitives 1 et 2 (état durable, résultat asynchrone) | grande |
| Les outils terminaux, l'arrêt sur deux erreurs identiques | 2205–2262 | primitive 1 | petite |
| **La composition du message système du chat** | `run_chat_turn`, `chat.rs` 1069–1118 | **aucune** : un gabarit | moyenne |
| L'annonce du plafond dans le fil | 1103–1110 | la tournure se déclare ; « une fois par seuil » et le plafond restent en Rust | petite |
| Les outils offerts au chat (`OutilsDuChat::de`) | 1327–1360 | garde dure ; seuls les textes de refus sortent | moyenne |
| Lire avant d'écrire (les empreintes) | 1374–1440 | garde sur un effet de bord, état durable | petite |
| La compaction avant le tour, et sur débordement | 291–363, 867–879 | primitives 1 et 3, le verrou `busy` | moyenne |
| L'après-tour : bilan, publication, rangement | `chat_turn_with`, 885–1003 | primitives 2 et 7 (fin de tour, attente, débit) | moyenne |
| L'atterrissage (commit, dossiers de masse exclus, avance rapide ou retenue) | `land.rs` 151–203, 285–381 | effets de bord gardés, registre « une fois » ; l'identité git reste en Rust | grande |
| Le niveau de la porte (bac, juge, cartes, classe, installation) | `gate.rs` 339–400 | un graphe de `Decide` ; l'octroi et `by.kind` restent en Rust | moyenne |
| **La table des classes de commandes** | `gate.rs` 80–190 | **aucune** : un fichier de règles, repli Rust en `system` | moyenne |
| **Le juge de la porte** | `served_judge`, 250–315 ; `decider.rs` | **aucune** : un `Decide` à deux options et un seuil ; le registre et les accords de racine restent en Rust | moyenne |
| Les commandes en attente d'accord de l'atelier | `atelier_fichiers.rs` 264–380, `card.rs` | primitive 2 (décision humaine, attente), état durable (les demandes vivent en mémoire) ; l'octroi reste en Rust | moyenne |

### Le terminal (`tty/src`)

Le terminal lit un seul flux, `GET /ide/flux?sujets=agent,tokens,mailbox,declarations`, repris par `Last-Event-ID`. Il
écrit par des `POST` JSON, surtout `/ide/teammate/mailbox`. Un terminal déclaré s'appuierait sur ces mêmes routes.

| Point | Où | Ce qui bloque | Taille |
|---|---|---|---|
| La machine à états de l'agent (attend, réfléchit, écrit, agit, fini) | `vue.rs` 25–38, 355–397 | une machine à états décrite (un `stateDiagram` Mermaid) | grande |
| La table événement → réaction du flux | `Vue::trame`, 533–745 | une table événement → réaction ; les jetons restent compilés | grande |
| L'aiguillage des lignes de boîte (écrite, masquée, gardée, fin de tour) | `ligne_de_boite`, 748–965 | une table prédicat → action | grande |
| Les cartes en attente, la plus ancienne d'abord | `boite.rs` 171–226 | les champs d'une carte sont en dur | moyenne |
| Répondre à une carte | `vue.rs` 1095–1106, 1221–1250 | **garde dure** | moyenne |
| L'en-tête de l'utilisateur, après une frappe seulement | `tui.rs` 44–58, `client.rs` 28–48 | **garde dure** | petite |
| Le clavier | `tui.rs` 858–900, `vue.rs` 1086–1190 | un format de clavier avec ses gardes | moyenne |
| Les commandes `/` et `!` | `commandes.rs` 11–105, `tui.rs` 533–690 | un catalogue commande → route → gabarit de rendu | moyenne |
| La ligne d'état et la ligne vivante | `vue.rs` 1279–1440 | un gabarit de texte choisi par état | moyenne |
| Ce qui monte dans l'historique | `vue.rs` 441–530, `tui.rs` 907–937 | chemin chaud, et un gabarit de vue | grande |
| Le filtre des trames | `tui.rs` 61–67 | un abonnement déclaré | petite |
| Les minuteurs, la cadence | `vue.rs` 431–440, `tui.rs` 316–360 | chemin chaud : des paramètres, pas un graphe | petite |
| La visionneuse d'appels | `visionneuse.rs` 29–55 | un format de clavier | petite |
| Le thème | `theme.rs` | déjà déclaré ; reste à le faire servir par l'hôte au lieu de l'embarquer | petite |

## L'ordre proposé

**Phase 0, sans primitive neuve**, chaque point avec son témoin, la sortie à l'octet près contre le Rust qu'il remplace :
1. le juge du contenu de l'équipier en `Decide` et `Route` ;
2. le juge de la porte en `Decide` à deux options (c'est aussi le ticket du juge hors du trait `Llm`) ;
3. la table des classes de commandes en fichier de règles ;
4. les appels d'outils écrits dans le texte, en nœud sur `model_out` ;
5. le message système du chat et le message de frontière de l'équipier, en gabarits ou en nœuds scriptés ;
6. le doublon Rust du bloc « Previous step », retiré ;
7. le thème du terminal servi par l'hôte.

**Phase 1, les primitives qui débloquent le plus** : les nœuds de boîte (5), la chaîne de gardes (4), la reprise avec
erreur typée (3), puis l'attente d'événement avec échéance (2).

**Phase 2, la fin d'un pas d'équipier**, la plus utile :
- « est-ce fini ? » en chaîne de gardes ;
- la porte de vérification ;
- les refus du fournisseur ;
- l'attente d'une question ouverte ;
- l'après-tour (bilan, rangement) sur la fin d'un run ;
- le plan en `ReactTransitionNode`.

**Phase 3, la boucle elle-même**, avec la primitive 1 :
- les budgets et les relances de la boucle de l'agent ;
- puis `teammate_loop` ;
- l'aiguillage asynchrone des outils en dernier.

**Phase 4, le terminal** : la machine à états, la table des événements et les gabarits de vue et de clavier. Le rendu
des jetons reste compilé.

**À chaque déplacement** :
- le témoin du comportement d'avant reste vert ;
- le coût d'un pas est mesuré avant et après (règle du 19 novembre : une mesure qui décide se fait en release) ;
- une garde dure n'entre dans un graphe que comme nœud dont le corps reste en Rust.
