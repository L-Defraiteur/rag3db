# Un modèle Vertex déclaré garde son jeton d'une heure

- **État** : ouvert ; constat, pas de correctif.
- **Famille** : produit
- **Gravité** : haute pour toute passe longue. Au bout d'une heure, chaque appel au modèle rend 401, et la
  décision est perdue sans que la passe s'arrête.
- **Atteignable en service** : oui. Tout hôte ou toute passe qui garde un `models.llm` de protocole `vertex`
  plus d'une heure : l'archiviste, l'équipier, le chat.
- **Touche rag3weaver** : oui, `model_source.rs` (`connect_llm`), `openai_llm.rs` (`OpenAiLlm::vertex`) et
  `gcp_auth.rs` (`TokenSource`).
- **Pour** : everything declarative, qui tient `models.llm` (confirmé par c7, 16 novembre).
- **Signalé par** : banc, mesure longue de l'archiviste (F300, 16 novembre).
- **Ouvert le** : 16 novembre 2026

## Reprise ED après audit du 16 novembre

Préparation autorisée par C7, sans Cargo ni cloud pendant la bascule G/Arbre.
Branche `cx-ed-vertex-token-refresh-r1`, WT portable
`/home/lucied/git_workspaces/rag3db-cx-ed-vertex-token-refresh-r1`, base927941a67.
Fichiers réservés proposés : `src/model_source.rs`, `src/openai_llm.rs`,
`src/gcp_auth.rs` ; pas `model_source/activation.rs` réservé à embarquement.
Plan : [préparer le renouvellement Vertex](../../everything-declarative/16-11-2026/03-preparation-vertex-token.md).
Le rouge HTTP reste à écrire puis à exécuter avant tout correctif ; aucune
preuve du banc n'est repeinte ni aucun vert annoncé sur cette préparation.

Après publication du témoin NON JOUÉ et PIN FINI G : les deux commits sont
repris sans changement produit sur base68590270 dans la branche neuve
`cx-ed-vertex-token-refresh-r2`, tête préparatoireb402243bb7 ; rapport03
actualisé. Le témoin de transport est écrit, son rouge reste NON OBSERVÉ,
le raccord déclaré et le correctif restent à faire. Aucun Cargo lancé.

## Le constat

La passe F300 de la mesure longue (`~/.cache/rag3db-banc-notes/compaction/long/F300-ov10-0653-20261116-0157/`
à luciepc) a tourné de 01 h 57 à 02 h 58. Le backend de la passe y garde un Decide sur
`google/gemini-3.8-flash` par Vertex. Les 45 derniers tours (199 à 244, sauf 208) rendent tous :

```
Decide (google/gemini-3.8-flash) : llm model error: HTTP 401: … "reason": "ACCESS_TOKEN_EXPIRED" …
```

Les requêtes refusées sont gardées dans `etat/requetes-refusees/*-401-*.json`. La passe ne s'arrête pas :
`appels.jsonl` marque ces tours `"ok": true`, avec l'erreur sous `reponse.errors`.

## Pourquoi

- `connect_llm` (`model_source.rs`), pour le protocole `vertex`, prend un jeton une seule fois :
  `TokenSource::from_env().and_then(|s| s.token())`.
- Il le pose en `Auth::Bearer` fixe dans `OpenAiLlm::vertex` (`openai_llm.rs`).
- Le commentaire de `connect_llm` le dit : « **Limite** : il est pris à la connexion et dure une heure ».
- `TokenSource` sait pourtant renouveler (`token()` relit le cache et refait l'échange, `invalidate()` sur un
  401), mais personne ne le rappelle.

## La forme attendue

- Le client Vertex garde la `TokenSource` et non le jeton. Il demande `token()` à chaque appel : le cache rend le
  même jeton tant qu'il est loin de son expiration.
- Sur un 401, il appelle `invalidate()` puis retente une fois.
- Un 401 qui persiste est une erreur nommée, qui arrête la passe, au lieu d'une décision perdue.

## La preuve

- Un témoin rouge d'abord : une `TokenSource` de test au jeton de courte durée, ou un faux serveur qui rend 401
  pour un jeton périmé. Un appel au-delà de cette durée doit réussir, et il rougit sur le code d'aujourd'hui.
- Puis la passe F300 rejouée par la mesure longue : 244 décisions, aucun 401.

## Ce que ça coûte au banc

F300 n'a que 199 décisions valides sur 244. Elle se juge sur ses 199 premiers tours, en le disant, ou se rejoue
après le correctif.

## Reprise ED autorisée après TIES, avant complément du rouge

Auditmasterdf2a5789888d16247f0dad42dcdaa3c4ceea2094 : aucun changement model_source.rs/openai_llm.rs/gcp_auth.rs depuisbase685 ; WIPc642propre, rebase3patches= souscx-ed-vertex-token-refresh-r3 (tête156986b340). Deuxjuges NONJOUÉS conservés ; complément prévu AVANTfix : toujours401→UNE invalidation/deuxrequêtes/erreurnommée, APIkeyBearerfixeinchangée, SSEcommencépasrejoué. Frontièretransport actuelle(snapshotfake→vraiVertexconstructeur) explicite, pasencoreunjugeconnect_llm déclaré. [Plan et bornes de fichiers avec Embarquement](../../everything-declarative/16-11-2026/03-preparation-vertex-token.md). AucunCargo/vraiVertex/credential/test lancé ; GOrecetteborneeC7encorerequis.

## Préparation corrective après rouge conservé

RougeCPUfe40/eb026 conservépreuves4d54cf4a47, aucunretry. C7 demande la préparation du transport à TokenSource conservée et du raccord déclaratif ; [plan17 et juges avantimpl](../../everything-declarative/16-11-2026/17-correctif-vertex-token.md). Deux nouveaux juges NONJOUÉS, sept fonctions prévues ; GOvert distinct nécessaire. Ticket reste ouvert.
