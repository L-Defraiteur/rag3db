# Lot 3 ter — le rappel tiré du prompt, sans redite

7 novembre 2026, rag3weaver archi. Retours de la session hooks-essai sur le binaire du 3 bis, et
décisions de Lucie relayées par c7.

## 1. La mesure : le plein texte flou, seul

Le rappel échouait deux tours sur trois sur une base de deux fiches, alors que le même rappel par
le MCP prenait 0,36 s. Le journal des runs de l'hôte, nœud par nœud du graphe
`search_structured` (backend `c6dd17bae70f`), dit où passe le temps :

| requête envoyée par le hook | `source` (embarquement) | `bm25` (plein texte) | `vector` |
|---|---|---|---|
| une notification de tâche, 418 car. | 51 ms | 1 944 ms | 19 ms |
| un message de session, 721 car. | 132 ms | 13 158 ms | 21 ms |
| un message de session, 789 car. | 110 ms | 14 561 ms | 22 ms |
| un message de session, 1 127 car. | 63 ms | 32 936 ms | 21 ms |
| « identity », 1 mot | 1 ms | 2 ms | 11 ms |

Le hook envoyait le prompt entier. Le plein texte flou, un contains relâché par mot en OU, coûte
selon le nombre de termes, et pas selon la base. « À froid » était donc faux. Le nœud continue
aussi après l'abandon du client : c'est un ticket de l'hôte
(`docs/tickets/2026-11-07-une-recherche-continue-sur-l-hote-apres-l-abandon-de-son-client.md`). La
borne côté moteur est à recherche (`docs/tickets/2026-11-07-le-plein-texte-explose-sur-une-requete-longue.md`).

## 2. La requête : des mots-clés, sans modèle

`src/rappel_du_prompt.rs`. Lucie : pas d'appel de modèle, trop lent dans le tour.

- **Les enveloppes tombent** : `<system-reminder>` entier, les en-têtes d'un
  `<cross-session-message …>` (son contenu reste), une `<task-notification>` réduite à son
  `<summary>`, toute autre balise, et la phrase du harnais « This came from another Claude
  session… ».
- **Sont gardés, par priorité** : un mot du vocabulaire de la mémoire (aujourd'hui les noms des
  fiches que la session a déjà reçues), le code entre accents graves, les identifiants
  (snake_case, camelCase, `a::b`, un nom de fichier, le dernier segment d'un chemin, un mot à
  tiret), les sigles (`GPU`, `HNSW`), les termes capitalisés hors début de phrase, puis les mots
  les plus longs.
- **Tombent** : les uuid, les hashes et ids à chiffres, les nombres, les options `--x`, les mots
  de moins de quatre lettres et les mots courants, en français et en anglais.
- Dédoublonnés sans casse, **au plus `recall_max_words` = 10**, rendus dans l'ordre du prompt.
- Une enveloppe sans contenu ne cherche rien : « rappel : rien à chercher ».

**D'où vient N.** Le témoin e2e mesure, sur un vrai gabarit memory à deux fiches et un message de
1 002 caractères, le nœud `bm25` :

| requête | `bm25` |
|---|---|
| le prompt entier | 18 379 à 19 274 ms |
| 4, 8, 10 ou 12 mots extraits | 11 à 62 ms |
| 16 mots extraits | 27 à 69 ms |
| 24 mots extraits | 74 à 141 ms |
| 40 mots extraits | 246 à 275 ms |
| les 16 premiers mots du contenu, sans tri | 587 à 673 ms |

Les prompts des témoins sont **synthétiques**, de la même forme que les quatre du tableau : une
notification de tâche, et des messages de session à session avec ids, chemins et uuid. Les vrais
sont des messages de sessions réelles. Ils restent hors du dépôt, et le classifieur de
permissions a refusé à juste titre de les copier en données de test.

Jusqu'à 16 mots, le coût reste dans le bruit. Dix mots laissent de la marge, et les mots longs
non triés coûtent plus que les mots triés.

## 3. Le budget : une seconde

`recall_timeout_ms` passe de 3 000 à **1 000**. À requête courte, le rappel complet prend 13 à
70 ms sur l'hôte en service (17:05:53, 17:06:46, 17:07:06), et 100 à 300 ms dans le témoin.
Un dépassement donne « Rappel des fiches : pas de réponse en 1000 ms — le rappel sera retenté au
prochain tour ». Le fils détaché qui finit le rappel en fond est le 3 ter (b).

## 4. Rien de ce que la session a déjà reçu

Lucie : « ne pas lui rebalancer des trucs qu'il a déjà eus ».

- Un fichier par session, `<dossier du journal>/hooks/<session_id>.json`, soit
  `~/.local/state/rag3weaver/hooks/<session_id>.json`. Il garde le tour courant et, par fiche
  rendue, le tour et l'empreinte de ce qui a été montré (nom et résumé).
- Une fiche déjà rendue et inchangée ne revient pas. Si tous les premiers résultats sont connus,
  c'est le **silence complet**, sans même une ligne. Une fiche dont le résumé a changé revient
  avec « mise à jour depuis le tour N ».
- **La compaction** vide le contexte de l'agent. Le bloc de `SessionStart` (source `compact`)
  rappelle en une ligne les fiches qui avaient compté, puis le déjà-vu repart de zéro.
- Un resume garde l'état, et un rejeu ne l'écrit jamais.

## 5. Le plein texte décide, le dense seul n'est pas jugé (3 ter b)

Trois tours réels de hooks-essai, le 7 novembre, ont montré le défaut du (a). Le hook lisait
`chunk.score`, le score du morceau affiché, et ignorait `signals`, où chaque signal donne le sien.

| tour | « régime GPU » | ce que le (a) a fait |
|---|---|---|
| 18:15:45, hors sujet | dense 0,32 | silence, sous le seuil |
| 18:16:13, une notification | dense 0,35 ; la fiche propre au plein texte, 1,66 | rendue sans seuil, hors sujet |
| 18:16:39, en plein sujet | dense 0,37 ; plein texte 1,386, au premier rang | silence : le hook lisait 0,37 |

La règle du (b) :
- **Les signaux se lisent par ligne** : `bm25`, le plein texte en score brut, et `vector`, le
  dense de 0 à 1. Sans `signals`, le score du morceau tient lieu.
- **La fiche propre est ôtée d'abord**, avant tout jugement.
- **Le plein texte fait le candidat** : les mots extraits du prompt touchent la fiche, son titre
  ou ses apports.
- **Le dense seul n'est pas jugé.** Le journal dit « dense seul, non jugé », et la fiche n'est
  pas rendue. C'est une attente de mesure, pas une doctrine. Le seuil dense de 0,50 du 3 bis n'a
  jamais été mesuré, et la clé `recall_min_score` reste lue sans effet. Un seuil dense ou un écart
  relatif se mesurera sur une mémoire peuplée (96, M4), à partir de prompts réels étiquetés « en
  rapport » ou « sans rapport ».
- **Au plus un rendu par tour**, le meilleur au plein texte, avec ses deux scores. C'est une
  politique, pas un seuil : l'agent a sa mémoire au moment d'agir, sans être inondé. Les autres
  que le plein texte touche vont au juge, en fond.
- **Le déjà-vu porte sur les candidats**, et tout rendu se marque. Le tour de 18:16:39 disait
  « déjà rendu : 0 » parce que la fiche, sous le seuil, était écartée avant le déjà-vu.
- **Un tour de notification ne cherche rien** : une `<task-notification>` sans parole
  d'utilisateur. Le journal dit « tour de notification, rien à chercher ».
- **La fiche propre** est trouvée par la fiche qui cite le handle `session:<uuid>` (genre
  `handle`) et porte un apport `identity`. Une Ref non vérifiée suffit. Sans identité, rien n'est
  exclu, et le journal dit « identité inconnue, pas d'exclusion ».
- **La requête**, d'après le premier tour réel de c7 :
  - le vocabulaire de la mémoire passe d'abord. C'est un cache commun aux sessions,
    `hooks/vocabulaire.json`, fait des mots des noms de fiches vus dans les réponses, sous le seuil
    compris ;
  - ensuite les mots courts et les nombres du code : `lib`, `43`, `WAL`, `L6`, `7970`. Seule une
    liste d'arrêt tombe ;
  - un jeton `A=4096` se coupe au `=` ;
  - un hash entre accents graves tombe comme un hash.
- La cause amont, un score qui ne se compare pas d'un signal à l'autre, est un ticket pour
  recherche :
  `docs/tickets/2026-11-07-search-structured-le-score-d-une-ligne-n-est-pas-comparable-d-un-signal-a-l-autre.md`.

## 6. Le journal

Une ligne par rappel dit la requête mot à mot, sa taille et la taille du prompt. Une autre dit la
durée. Une dernière donne l'issue : « exclu : « x » (fiche propre) ; sous le seuil 0.50 : « y »
0.45 ; déjà rendu : 1, neuf : 0 ; silence ». La ligne d'en-tête porte le commit du bâti
(`commit 23d8d3c5a`, suivi de `+` si l'arbre était sale), en plus de l'empreinte par mtime que
l'hôte compare.

## 7. Le 3 ter (b) : le rappel fini en fond, et son juge

- **Le fils détaché.** Le budget d'une seconde dépassé, le hook écrit une tâche à côté de l'état
  de la session, `<session_id>.tache.json`. La tâche porte la requête, le prompt nettoyé, le tour
  et le chemin du réglage. Le hook lance alors `hook --rappel-differe <tâche>`, détaché de son
  groupe de processus, entrée et sorties fermées. Le fils refait le rappel sous 30 s et juge les
  candidats. Il range le résultat dans `<session_id>.differe.json` et journalise sous `[fond]`.
  Jamais en rejeu.
- **Le juge** (`juger`) fait un appel structuré à température nulle, sous un schéma strict
  `{pertinent, pourquoi, items_gardes}`. Il garde seulement ce qui sert au message de ce tour, et
  dans le doute il ôte. Un nom qu'il rend sans l'avoir reçu est jeté : il ôte, il n'invente rien.
  Son bilan va au journal : « juge : 2 candidat(s), 1 gardé(s) — pourquoi ». Le modèle est celui
  du régime, Vertex par `GOOGLE_CLOUD_PROJECT`. Sans modèle, ou avec `recall_judge: false`, les
  candidats passent tels quels, et le journal le dit.
- **Ce qui part au juge** : les candidats que le plein texte touche, au-delà du premier rendu.
  Le fils ne refait pas le rappel pour eux, et le juge ne répète jamais une fiche déjà rendue.
- **Le rendu.** Le résultat tombe au premier `PostToolUse` du tour suivant, en
  `hookSpecificOutput.additionalContext`, sous « Rappel du tour d'avant (requête « … ») ». Ce
  `PostToolUse` n'ouvre pas l'hôte et ne lit qu'un fichier. Le résultat passe sous le déjà-vu et
  n'est rendu qu'une fois. Faute de `PostToolUse`, il tombe en tête du prompt suivant. Des
  candidats hors sujet donnent le silence, avec la raison du juge au journal.
- **PostToolUse dans les settings.** `scripts/hooks_claude_code.sh --poser` pose maintenant six
  entrées, dont `PostToolUse` avec un délai de 5 s. Claude Code fige ses hooks au démarrage :
  hooks-essai ne l'aura qu'à la prochaine relance de Lucie. D'ici là, le différé tombe au prompt
  suivant.
- **La dépense du juge (3 ter c).** Le juge appelle un modèle facturé, depuis la session de Lucie.
  Il n'est appelé que si au moins deux fiches neuves sont touchées au plein texte, ou si le
  rappel dépasse son budget.
  - `"recall_judge": false` dans `hooks.json` le coupe, au tour suivant. Les candidats venus du
    tour se taisent alors. Pour un rappel refait, seul le premier au plein texte passe, avec la
    même politique d'un seul rendu.
  - `recall_judge_max_per_hour`, 20 par défaut, toutes sessions comprises, est une **politique de
    dépense déclarée**, pas un seuil mesuré. Au-delà, le juge est sauté, et le journal dit « sans
    juge (plafond horaire atteint) ».
  - Chaque appel se journalise : `[fond] juge : modèle …, … ms, entrée … jetons, sortie … jetons`.
    La facture se relit ainsi dans `hooks.log`.
- **Le plafond d'items par bloc** reste celui de la requête, trois fiches. `hooks.log` n'a encore
  que cinq rappels, de deux candidats chacun : trop peu pour mesurer mieux.

## 8. Le 3 ter (d) : l'hôte reçoit l'échéance du client

hooks-essai a ajouté `deadline_ms` à `POST /call`, dans le lot chat-et-delai (682655352). Passé ce
délai, l'hôte arrête le run par le chemin d'arrêt commun, avec `cancelled_by: "deadline"`. Il
répond en 422, avec `error_kind: "deadline"` et `transient: true`. Le client du hook s'en sert
désormais :
- chaque `/call` porte `deadline_ms`, à **95 %** du délai que le client tient. Le rappel du tour
  borné à 1 s envoie 950 ms, les autres appels sous 5 s envoient 4 750 ms, et le fils du rappel
  différé, sous 30 s, envoie 28 500 ms. La réponse « délai dépassé » arrive ainsi avant que le
  client ferme ;
- le client lit le 422 par `error_kind`, pas par son texte. Le journal dit « délai dépassé côté
  hôte », et le rappel confie alors sa requête au fils, comme pour un délai du client.

La limite reste celle du ticket de hooks-essai, « un arrêt ne coupe pas une requête du moteur en
cours ». L'arrêt se lit entre deux nœuds du graphe, et une requête du moteur déjà lancée va à son
terme. L'extraction des mots-clés du § 2 reste donc la vraie garde contre un plein texte long.

## 9. Le 3 ter (e) : le vocabulaire sans les bilans, une empreinte stable

Le premier tour sur le binaire posé le 8 novembre, chez hooks-essai à 8 h 33, a extrait
« hooks-essai sur avec rend Mais mémoire brut disparu donc correctif ». Des mots d'arrêt étaient
passés en tête, parce que le vocabulaire de la mémoire les tenait pour connus. Le cache prenait les
mots de **tous** les noms de fiches, et le nom d'une fiche de bilan du juge est le prompt entier de
son run. Ticket de hooks-essai :
`docs/tickets/2026-11-08-les-bilans-du-juge-remontent-au-rappel-sur-des-mots-courants.md`.

Ce que le (e) prend :
- **Le vocabulaire n'admet ni un mot d'arrêt, ni un mot de moins de quatre lettres** sans forme de
  code (`rappel_du_prompt::utile_au_vocabulaire`). Les noms `bilan : …` et `outil : …`, générés
  par le juge, ne l'alimentent pas.
- **Un mot connu ne passe plus outre la liste d'arrêt.**
- **Le cache existant se nettoie à la première lecture** : les mots qui n'y ont pas leur place
  sortent, et le fichier est réécrit, sauf en rejeu.
- **Le nom d'une fiche rendue est borné à 200 caractères**, et à 120 dans le journal.
- **L'empreinte du déjà-vu passe à blake3**, sur 32 chiffres hexadécimaux, stable d'un binaire à
  l'autre. `DefaultHasher` ne l'est pas d'une version de Rust à l'autre, et c'est l'une des deux
  causes possibles du point 3 du ticket. Une empreinte d'avant, de 16 chiffres, se lit « déjà vue »
  et se réécrit au prochain rendu. La relance de l'hôte ne rend donc rien « neuf » à tort.

- **Les fiches du juge ne sont plus rappelées en tête de tour** (A, tranché par hooks-essai). Une
  fiche de bilan ou d'outil est un compte rendu, pas un savoir. Elle reste lisible par
  `get_fiche`, et le journal la nomme : « exclu : « … » (bilan du juge) ». On n'a pas pris B, la
  confier au seul juge différé : sans modèle dans l'environnement du hook, B revenait à la taire
  sans le dire. Le critère :
  - `via: juge` quand la ligne du rappel le porte ;
  - sinon le nom que le juge leur donne, `bilan : …` ou `outil : …`.

  `recall_fiches` ne rend pas `via` aujourd'hui : ce champ n'est pas dans les `returnFields` de
  `Fiche` du gabarit memory. Son ajout est demandé à 96 ; le manifeste de Lucie est le sien. Le
  préfixe tient lieu en attendant.
- **Le second ticket de hooks-essai**, « une fiche déjà vue redevient neuve après la relance de
  l'hôte », est fermé par l'empreinte blake3.

Ce que le (e) laisse au ticket des bilans : **le nom court d'une fiche de bilan**, un titre et
l'empreinte de la consigne, côté juge. hooks-essai le prend dans un lot à part.

La passe est épinglée sur la lib dadbf0041, la lib ordinaire du 8 novembre, sans
`RAG3WEAVER_MOTEUR_ANCIEN`.

## 10. Le 3 ter (f) : `recall_min_score` retiré

Depuis le (b), la logique du rappel ne lit plus `recall_min_score`. Le plein texte fait le
candidat, et le dense seul n'est pas jugé. Le champ est donc retiré du réglage. La clé reste
acceptée dans `hooks.json`, sans effet, pour qu'un fichier d'avant reste valide, et le script
d'installation ne l'écrit plus. Recherche a ajouté `bestSignal: {label, score, rank}` à chaque ligne
fusionnée (d540813d5), le signal qui pèse le plus dans son rang. Un seuil, s'il en faut un, se
mesurera sur ce champ, à partir de prompts réels étiquetés « en rapport » ou « sans rapport », quand
la mémoire sera peuplée.
