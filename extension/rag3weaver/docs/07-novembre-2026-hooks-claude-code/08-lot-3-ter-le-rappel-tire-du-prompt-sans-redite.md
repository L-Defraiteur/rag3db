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

## 5. L'échelle et l'exclusion, dites

- Le seuil `recall_min_score` (0,50) vaut pour une similarité de 0 à 1. Le meilleur signal peut
  être le plein texte, dont le score brut dépasse 1 (3,53 dans le témoin, 27,92 sans le service
  d'embarquement). Sur une échelle brute, **pas de seuil**. Le journal dit « échelle brute (score
  au-delà de 1), pas de seuil », et le bloc dit « score brut du meilleur signal, sans seuil ».
  Trier ces candidats-là est le travail du juge du 3 ter (b).
- La fiche propre est exclue par sa clé. La clé vient de l'identité que le hook trouve : la fiche
  qui cite le handle `session:<uuid>` (genre `handle`) et porte un apport `identity`. Une Ref non
  vérifiée suffit, puisque `cited_by` ne regarde pas la vérification. Sans identité trouvée, rien
  n'est exclu, et le journal dit « identité inconnue, pas d'exclusion ». `whois` (lot 4) est la
  bonne lecture.

## 6. Le journal

Une ligne par rappel dit la requête mot à mot, sa taille et la taille du prompt. Une autre dit la
durée. Une dernière donne l'issue : « exclu : « x » (fiche propre) ; sous le seuil 0.50 : « y »
0.45 ; déjà rendu : 1, neuf : 0 ; silence ». La ligne d'en-tête porte le commit du bâti
(`commit 23d8d3c5a`, suivi de `+` si l'arbre était sale), en plus de l'empreinte par mtime que
l'hôte compare.

## 7. Le 3 ter (b), juste derrière

- Le budget dépassé lance un fils détaché, `hook --rappel-differe`, qui finit le rappel.
- Le juge Decide `{pertinent, pourquoi, items_gardes}` ôte et n'invente rien. Son bilan va au
  journal : « juge : 3 candidats, 1 gardé, pourquoi ».
- Le résultat tombe au premier `PostToolUse` sous « rappel du tour d'avant », ou au prompt
  suivant tant que les settings n'ont pas `PostToolUse`. Claude Code fige ses hooks au démarrage,
  et `--poser` l'ajoutera pour la prochaine relance de Lucie.
- Le plafond d'items par bloc sera mesuré sur `hooks.log`.
