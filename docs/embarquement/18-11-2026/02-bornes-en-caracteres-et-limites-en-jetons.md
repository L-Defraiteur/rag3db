# Embarquement : les bornes en caractères qui tiennent lieu d'une limite en jetons

Demande de Lucie, le 18 novembre à 15 h 4x, par c7 : « voir si d'autres bornages en jetons plutôt qu'en textes ailleurs
seraient pertinents ». Ce relevé fait suite à la panique du 7879 dans la fenêtre de G, à 15 h 32 : 11 000 « é », soit
11 002 jetons, ont produit « Expand 8194 != 11002 ». Le ticket est `embarquement/tickets/2026-11-15-un-plafond-par-texte-en-jetons-exact.md`.

Tout est lu sur master, à e0784a0d23 ; les chemins sont relatifs à `extension/rag3weaver/`. Deux agents de lecture ont
fait la cartographie, et j'ai revérifié les points qui pèsent : la troncature de bge-m3 et de granite, la validation de
`query_max_chars`, la doc du plafond du service. Les ratios et les fenêtres des modèles sont **mesurés** sur les
`tokenizer.json` et les `config.json` installés à luciepc. **Rien n'est codé** : c'est à c7 de le décider.

## 1. Le pire ratio, mesuré

Le ratio est compté en jetons par caractère Unicode, jetons spéciaux exclus. Il est mesuré sur 3 000 caractères, avec
la troncature levée.

| Texte | Famille XLM-R (bge-m3, granite-107m et 278m, multilingual-minilm, bge-reranker-v2-m3, mmarco) | minilm, msmarco (WordPiece) |
|---|---|---|
| prose française | 0,27 | 0,33 |
| code Rust | 0,30 | 0,29 |
| « é » répété, « aé » alterné, « é1! » | **1,00** | 0,00 (un seul [UNK] par mot géant) |
| CJK courant | 0,67 | 1,00 |
| emojis, ZWJ, drapeaux | **1,00** | 0,00 |
| hexadécimal | **1,00** | 0,00 |
| symboles `{}[]();…` | 0,95 | 1,00 |
| diacritiques combinants (zalgo) | **1,00** | — |
| chiffres, espacés, tabulations | 0,30 à 0,50 | — |
| retours à la ligne, CJK de l'extension B | ≈ 0 (fusionnés ou inconnus) | — |

**Ce qu'on en tire.** Sur les onze textes adverses essayés, aucun texte ne dépasse **1 jeton par caractère, plus 2
jetons spéciaux**. C'est un constat sur ces essais, pas une preuve. Par octet, le pire ratio vaut aussi 1, atteint par
l'ASCII. D'où une règle simple :

**Une borne en caractères (ou en octets) est sûre pour un modèle si elle ne dépasse pas sa fenêtre moins 2.**

À l'inverse, le « 3 caractères par jeton » du service (`CARACTERES_PAR_JETON`) ne vaut que pour la prose et le code.
Un texte dégénéré, en accents, en emojis ou en hexadécimal, coûte **3 fois plus** de jetons que prévu.

**Les fenêtres réelles :**

| Modèle | Fenêtre |
|---|---|
| bge-m3 | 8 194 positions, soit 8 192 jetons utiles |
| granite-107m, granite-278m | 514 positions, tronqué à 512 par le code |
| multilingual-minilm | 512 positions, tronqué à 128 |
| minilm | 512 positions |
| rerankers | 512 jetons (`MAX_SEQ_LEN`), même bge-reranker-v2-m3, qui a 8 194 positions |

## 2. Le tableau des bornes

| # | Borne (où) | Unité, défaut | Limite du modèle | Au-delà, aujourd'hui | Risque au pire ratio | Recommandation |
|---|---|---|---|---|---|---|
| 1 | Plafond du service, `MAX_CHARS_PAR_TEXTE` (`src/daemon/embeddings.rs:678`), `RAG3WEAVER_EMBED_MAX_CHARS` ; coupe `cut_to_cap` (`:643`) et côté client `within_cap` (`:1210`) | caractères, 12 000 (« ≈ 4 000 jetons » à 3 car./jeton) | bge-m3 : 8 192 jetons ; granite : 512 | coupe silencieuse comptée (`cut_texts`) | **bge-m3 : panique sur la carte** dès 8 193 jetons (11 000 « é ») ; service empoisonné, 503 nommé, arrêt en code 70. Et 12 000 jetons demandent 9 fois la mémoire d'attention prévue pour 4 000 | **Compter en jetons**, dans le service, qui a le tokenizer : voir § 3, point A |
| 2 | Troncature de l'embarqueur bge-m3 (`src/burn_bge_m3_embedder.rs:126`) | **aucune** (rembourrage seul) ; le `tokenizer.json` n'en porte pas non plus | 8 192 | panique « Expand 8194 != N » | panique | **Compter en jetons** : `with_truncation(max_length 8 192)`, compté par `troncatures()`, comme granite (point A) |
| 3 | Troncature de granite (`src/burn_granite_embedder.rs:36, 106`) | jetons, 512 | 512 | coupe comptée (`troncatures()`, avertissement du catalogue) | aucun ; coupe silencieuse mais comptée | laisser |
| 4 | Troncature de minilm et multilingual-minilm (`burn_minilm_embedder.rs:73`, `burn_multilingual_minilm_embedder.rs:110`) | jetons, 512 et 128 | 512 | coupe comptée | aucun | laisser |
| 5 | Le découpeur, `max_size` (`src/chunker.rs:69`, `src/config.rs:209`) | **octets** au test d'entrée et en `Fixed`/`Lines` ; caractères (non vérifié) dans `text-splitter` ; 1 500, recouvrement 200 | bge-m3 : 8 192 ; granite : 512 | rien ne coupe un morceau trop gros (`Lines` admet jusqu'à 3 fois `max_size`) ; le préfixe de titre (`title_max_chars` 256) n'est pas retranché | bge-m3 : sûr (au plus 4 500 jetons). **Granite : coupe à 512** dès qu'un morceau de 1 500 octets dépasse 0,34 jeton par octet (code dense, symboles, CJK) ; la fin n'est pas cherchable par le vecteur | **Estimer avec une marge**, puis mesurer : lire le compte `troncatures()` (déjà au journal d'index) sur un vrai corpus avant de changer quoi que ce soit (point C) |
| 6 | Budget de lot, `EMBED_CHAR_BUDGET` 8 192 (`src/embedder.rs:1069`), `RAG3WEAVER_EMBED_CHAR_BUDGET`, `lot_budget_si` (`:1145`), `burst::next_batch` (`burst.rs:325`) | octets (`text.len()`) estimés à 3 par jeton. Le service : budget **linéaire** (`budget_batches`, `daemon/embeddings.rs:500`) ou durée mesurée (`burst::next_paced_batch`). Le client local : aire d'attention `seq × jetons² / 2` (`stable_batches`) | la mémoire de la carte, pas une fenêtre | un texte seul au-dessus du budget forme son lot | **service : jusqu'à 3 fois** les jetons prévus par lot (aire bornée par budget × fenêtre) ; **client local : aire sous-estimée jusqu'à 9 fois** (jetons sous-estimés 3 fois, au carré). Saturation possible, pas de panique de positions | **Différé** (c7, 18 novembre) : voir le point B et son ticket |
| 7 | `query_max_chars` (`src/search.rs:987`), validation `query_max_chars ≤ context_tokens × 3` (`src/backend.rs:1516`) | caractères, 6 000 | celle du modèle de requête | coupe sur une frontière de mot, avec avertissement | la validation est **dans le mauvais sens** : au pire ratio, il faut `≤ context_tokens − 2`. Sans danger tant que le point A tient | **Estimer avec une marge** : valider à `context_tokens − 2` (sûr), ou garder ×3 en le nommant « estimation » (point D) |
| 8 | `HttpEmbedder`, fournisseur `compatible` (`src/http_embedder.rs:92`) | **aucune** | celle du fournisseur (8 192 jetons chez OpenAI, par exemple) | erreur du fournisseur | une passe en échec, nommée par le fournisseur | **Laisser**, avec une option : un `max_chars_per_text` déclarable au manifeste, appliqué comme `within_cap` (point E) |
| 9 | Rerankers burn (`burn_reranker.rs:49`, `burn_xlmr_reranker.rs:62`) ; `/rerank` coupé à 12 000 caractères (`daemon/embeddings.rs:866`) ; `DaemonReranker` sans borne côté client | jetons, 512, `OnlySecond` | 512 | passage coupé en silence ; une requête qui dépasse seule rend une `EmbedError` | aucun ; le plafond de 12 000 caractères n'y protège rien | laisser |
| 10 | Mémoire longue, `query_chars` 4 000 (`templates/backends/memory/graphs/archive.mmd:15`, `scripts/archive_query.rhai`) | caractères | bge-m3 : 8 192 | coupe nommée dans la trace | sûr : au plus 4 002 jetons | laisser |
| 11 | Débit `Rate { chars_per_second }` (`src/estimate.rs:261`) ; attente de 6 s (`backend.rs:1009`, sur des ms par fichier) | caractères par seconde | le coût réel est en jetons | — | un corpus dense coûte jusqu'à 3 fois plus que la prose au caractère : l'estimation et la conversion « 6 s en lots » du banc (pas encore sur master) se trompent du même facteur | **Estimer avec une marge** : le banc mesure son débit par type de corpus, ou en jetons par seconde (point F) |
| 12 | LLM : `ContextOverflow` en jetons (`llm.rs:897`, `anthropic_llm.rs:794`, `openai_llm.rs:1262`) ; budgets d'outils en octets et en caractères (`daemon_host/juge.rs:51` 24 Kio, `code_tools/grep_symbols.rs:7` et `symbol_navigation.rs:10` 96 Kio, `claude_code_hook.rs` 1 500, 3 000, 8 000, `RunCommandNode` 20 000) | divers | fenêtre du fournisseur | débordement détecté en jetons, puis une compaction (`agent.rs:1727`) ; budgets coupés et marqués | aucun : ces bornes sont des budgets d'affichage, pas des proxys de fenêtre | laisser |
| 13 | OCR (`src/burn_ppocr.rs:98`) | pixels | — | — | — | hors sujet |

**À corriger au passage**, sans lien avec les jetons :
- la doc de `Identite.max_chars_per_text` (`src/daemon/embeddings.rs:143`) dit « refusé (413 nommé) », alors que le
  code coupe depuis 828ff28153 ;
- `RunCommandNode` dit « caractères » mais compte des octets (`commande.rs:1422`) ;
- `MAX_SCHEMA_CHARS` (`openai_llm.rs:785`) mesure des octets.

## 3. Les recommandations, par ordre

**A. Le plafond du service en jetons, et la troncature de bge-m3** (bornes 1 et 2). C'est le seul **risque de
panique** ; il est atteint le 18 novembre, et c'est le ticket du plafond en jetons. Deux pas :
1. `burn_bge_m3_embedder.rs` tronque à 8 192 jetons, compté par `troncatures()`, comme granite. La panique devient une
   coupe dite.
2. Le service lit, pour chaque texte, son compte de jetons avant la troncature (`Encoding::get_overflowing()`, ou un
   encodage sans troncature). Il coupe à `max_tokens_per_text`, dit dans l'identité, à 4 096 par défaut pour bge-m3
   (la mémoire d'attention visée par les 12 000 caractères). Il le compte dans `cut_texts`.

Le plafond en caractères reste en garde grossière, avant tout encodage. Témoin, rouge d'abord : 11 000 « é » rendent un
vecteur et une coupe comptée, jamais une panique. **Taille : une demi-journée.**

**B. Les lots planifiés sur le vrai compte de jetons** (borne 6). **Corrigé le 18 au soir, et différé par c7.** La
première version de cette page disait « `par_lots` encode déjà » et « 9 fois » : c'est faux pour le service.
`par_lots` n'encode pas (c'est l'embarqueur qui encode), et son budget est linéaire. Un lot du service porte au pire 3
fois les jetons prévus, et l'aire reste bornée par budget × fenêtre ; depuis A, un texte ne dépasse plus la fenêtre. Le
« 9 fois » vaut pour `stable_batches`, côté client avec un embarqueur local. Le vrai compte demanderait un
`count_tokens` au trait, soit un second encodage. Ticket « souhait » :
`embarquement/tickets/2026-11-18-les-lots-d-embarquement-se-taillent-a-trois-octets-par-jeton.md`, avec la version
la moins chère (1 jeton par octet, sans encodage) et sa mesure de débit à faire d'abord.

**C. Le découpeur sous granite** (borne 5). Ne rien changer avant d'avoir mesuré. **La mesure, choisie le 18 au
soir** : depuis A, l'identité d'un service dit `truncated_texts`, que granite remplit par sa troncature à 512. Une fois
le binaire de A posé sur le 7878, son `/sante` comptera sur le trafic réel les morceaux coupés à 512 jetons ; il sera
lu le lendemain, sans index de mesure ni essai sur le service.

**Premier relevé, le 19 novembre** : la chauffe compte 169 troncatures à chaque démarrage du 7878. Sa forme de
380 « mots » fait 762 jetons ; le ticket est `embarquement/tickets/2026-11-19-la-chauffe-du-service-compte-ses-troncatures-comme-du-trafic.md`.
Une fois la chauffe retirée, il reste **67 morceaux du trafic réel coupés à 512 jetons en 40 minutes** (10 h 47 → 11 h 27).
Quels textes, et quelle vue : le service ne les nomme pas. C'est à lire côté client, au journal d'index des hôtes, qui
ne voit pas aujourd'hui les troncatures d'un service distant (`DaemonEmbedder::troncatures` n'est pas relayé).
S'ils sont nombreux, `text-splitter` sait dimensionner par un tokenizer, et `max_size` peut alors se dire en jetons
par modèle. **Taille : une mesure, puis à décider.**

**D. La validation de `query_max_chars`** (borne 7) : `≤ context_tokens − 2`, ou le message dit « estimation à 3
caractères par jeton ». **Taille : une heure.**

**E. Le fournisseur `compatible`** (borne 8) : laisser, ou un `max_chars_per_text` optionnel au manifeste. À faire
seulement si un fournisseur réel refuse.

**F. Le débit en caractères par seconde** (borne 11) : à dire au banc. Sa table doit distinguer la prose, le code et
le texte dense, ou mesurer en jetons par seconde. Sinon, la conversion « 6 s en lots » se trompe jusqu'à 3 fois sur un
corpus dense.

**Tout le reste** (bornes 3, 4, 9, 10, 12, 13) : **laisser**. Le modèle tronque lui-même en jetons et compte la coupe,
ou la borne est un budget d'affichage, ou elle est déjà sûre au pire ratio.

**Non vérifié**, à relire avant le lot A :
- les enveloppes d'activation (`model_source/activation.rs:1101-1161`) relaient-elles `troncatures()` et
  `char_truncations()` ?
- le dimensionneur par défaut de `text-splitter` compte-t-il en caractères ?
- les embarqueurs candle (`bge_m3_embedder.rs`, `candle_embedder.rs`) dépendent-ils du préréglage de leur
  `tokenizer.json` ?
