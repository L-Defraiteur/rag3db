# Les offres batch de Gemini sur Vertex

21 novembre 2026, soir. Demande de Lucie, relayée par c7 : se renseigner sur les offres batch de Gemini sur Vertex (elle
en a vu pour Argon et Neon). Rien de payant n'a été lancé pour cette page.

Les faits viennent de la documentation officielle de Google, lue le 21 novembre. **Les prix sont à revérifier sur la
page tarifaire vivante avant toute décision** : elle ne se laisse pas lire par nos outils (elle ne rend que son titre).
Les chiffres ci-dessous viennent des extraits de cette page et de celle de l'API Gemini, et les moteurs de recherche
notent que ces pages ont des dates incohérentes.

## Ce que c'est

- **Le dépôt** :
  - un fichier JSONL dans Cloud Storage, une requête par ligne, ou une table BigQuery (`bq://projet.dataset.table`) ;
  - un appel `POST …/projects/<projet>/locations/global/batchPredictionJobs`. Le point d'accès `global` est accepté ;
  - la sortie : un JSONL dans Cloud Storage, ou une table BigQuery dans la même région que le travail.
- **Le délai** : « 24 hours turnaround time ». La plupart des travaux finissent dans les 24 h **après leur départ, sans
  compter l'attente en file**, qui peut durer jusqu'à **72 h** avant expiration. Au bout de 24 h, ce qui n'est pas fini est
  annulé, et seules les requêtes finies sont facturées. Le délai réel n'est pas mesuré : il faudrait un essai.
- **Les limites** :
  - 200 000 requêtes par travail ;
  - un fichier d'entrée d'1 Go au plus ;
  - « no predefined quota limits » : la capacité est partagée, et les travaux attendent en file ;
  - le batch **n'est pas couvert par le SLA** ;
  - pas de débit provisionné, pas de RAG, pas de modèle Gemini 3+ réglé ;
  - en entrée BigQuery, ni compte de service personnalisé ni CMEK.
- **L'échec partiel** : une ligne par requête en sortie. Une réponse réussie est dans `response`, une erreur dans
  `status`. Le service réessaie de lui-même, sans dire comment. Pour reprendre, on relance un travail sur le même fichier :
  il ne traite que les requêtes manquantes ou en échec, et fusionne avec les résultats précédents.

## Les modèles et les prix

| modèle | en ligne, entrée / sortie (par million de jetons, global) | batch | lecture en cache | accepté en batch |
|---|---|---|---|---|
| **Gemini 4 Argon** | 2 $ / 10 $ jusqu'au 30 nov. 2026, puis **4 $ / 20 $** dès le 1er déc. | moitié prix | 0,10 $, puis 0,20 $ | oui |
| **Gemini 3.8 Flash** | 0,75 $ / 3,75 $ jusqu'au 31 déc. 2026, puis 1,50 $ / 7,50 $ | 0,375 $ / 1,875 $, puis 0,75 $ / 3,75 $ | 90 % de remise | oui |
| **Gemini 4 Neon** | 3 $ / 15 $ (une ligne de la page ; d'autres lignes diffèrent) | — | — | **non** : absent de la liste des modèles du batch |

Hors de la région `global`, les prix sont environ 10 % plus chers.

**Le cumul avec le cache** : pour Gemini 3 et après, les remises du cache et du batch **se cumulent**, soit 95 % de remise
sur les jetons d'entrée en cache. Pour Gemini 2.5, elles ne se cumulent pas. Le cache explicite n'est pas accepté en
batch, seul l'implicite l'est.

**À savoir** :
- Neon et Carbon répondraient 403 sur Vertex pour certains comptes de facturation, d'après un fil du forum de Google ;
- un autre fil signale que les **logprobs sont coupés depuis le 20 nov. sur les Gemini 4** (400 INVALID_ARGUMENT).

Ce sont des rapports d'utilisateurs, pas des annonces de Google. Ils ne touchent pas nos juges Gemini, qui lisent une
lettre et non des logprobs.

## Notre usage

| appel | peut passer en batch ? | pourquoi |
|---|---|---|
| mes mesures du banc (juges sur 120 cas, grilles, rubriques) | **oui** | le résultat sert dans l'heure ou le lendemain ; un travail en file 24 h ne gêne pas |
| l'archiviste (extraction, découpe) | **oui**, pour le rattrapage et les passes de nuit | rien n'attend sa réponse en direct |
| l'étiquetage des messages en différé | **oui**, s'il ne sert pas la décision immédiate | l'étiquette rangée sert aux décisions suivantes |
| la première porte (Qwen local) | sans objet | locale, 0,1 s |
| les juges Gemini en direct, le survivant Argon | **non** | la carte attend entre 30 s et 10 min, pas 24 h |

**L'économie, en chiffres mesurés** :
- **ma passe Gemini Flash de cet après-midi** : 960 appels, 308 592 jetons d'entrée et 99 908 de sortie (raisonnement
  compris). C'est **0,60 $ en ligne** et 0,30 $ en batch, au prix d'introduction. Une journée de banc chargée (environ
  3 000 appels de juge) coûte 2 à 3 $ en ligne et gagne 1 à 1,50 $ en batch ;
- **le survivant Argon** (122 cas, jusqu'à 3 appels par cas, environ 600 à 800 jetons par appel) : quelques dollars au
  plus. Mais il **doit** rester en direct ;
- **l'archiviste** est le seul poste où le batch pèserait. Son volume réel par jour est chez la session mémoire : la
  remise est de 50 %, et de 95 % sur ses préfixes en cache.

**En dollars, l'enjeu de nos mesures est petit.** Le batch vaut surtout pour l'archiviste et les rattrapages de nuit.

## Le cache de préfixe pour le direct

- **Le prix d'un jeton en cache** : 90 % de remise sur l'entrée (0,10 $ le million pour Argon au lieu de 2 $). Pas de
  coût de stockage pour le cache implicite, et l'écriture se paie au prix normal de l'entrée.
- **La taille minimale** : 6 144 jetons de préfixe commun pour Gemini 3.8 Flash (cache implicite seulement), 4 096 pour
  les Gemini 4.
- **La durée de vie** : « no later than 24 hours », et la plupart des entrées sont évincées plus tôt. Aucune durée n'est
  garantie pour l'implicite. Le cache explicite a 60 min par défaut, avec un coût de stockage, mais il n'existe ni pour
  Argon ni pour Neon.
- **Notre préambule de juge** fait **331 jetons de médiane** (388 au plus) : loin des 6 144. **Il ne profite pas du
  cache.** Le gonfler artificiellement coûterait plus qu'il ne rapporte. Seul l'archiviste, dont les consignes et les
  segments dépassent ce seuil, y gagne. Le banc avait mesuré, le 19 nov., qu'il frappe le cache en tout ou rien.

## Une disponibilité à surveiller : Argon

Pendant la rédaction de cette page, **le survivant joué par Argon en direct reçoit des 429 en série** (« Resource
exhausted », capacité partagée, aucune limite annoncée). Six tentatives espacées de 30 s à 4 min ne suffisent pas
toujours. Pour un garde placé sur le chemin de chaque commande refusée, c'est un risque de disponibilité, que le batch
n'aide pas. La mesure complète sera dans la page du survivant.

## Un essai réel, si Lucie le veut

**Rejouer en batch ma passe Gemini Flash de cet après-midi** : 960 requêtes, environ 0,30 $.

Il faut :
- un seau Cloud Storage dans `lr-hub-472010`, à créer, sans suppression douce (soft delete), qui coûterait du stockage ;
- que l'agent de service « AI Platform » ait le droit de le lire.

On mesurerait :
- le délai réel, de la file à la fin ;
- les lignes en échec ;
- l'identité des lettres avec la passe en ligne.

C'est à demander à Lucie avant tout lancement.

## Les sources

- [Batch inference for Gemini](https://docs.cloud.google.com/vertex-ai/generative-ai/docs/multimodal/batch-prediction-gemini)
- [Batch inference from Cloud Storage](https://docs.cloud.google.com/vertex-ai/generative-ai/docs/multimodal/batch-prediction-from-cloud-storage)
- [Batch prediction API](https://docs.cloud.google.com/vertex-ai/generative-ai/docs/model-reference/batch-prediction-api)
- [Context caching overview](https://docs.cloud.google.com/vertex-ai/generative-ai/docs/context-cache/context-cache-overview)
- [Pricing (Agent Platform)](https://cloud.google.com/vertex-ai/generative-ai/pricing), [Gemini Developer API pricing](https://ai.google.dev/gemini-api/docs/pricing)
- [Gemini 4 Argon available on Google Cloud](https://cloud.google.com/blog/products/ai-machine-learning/gemini-4-argon-available-on-google-cloud), [Gemini 3.8 Flash](https://docs.cloud.google.com/gemini-enterprise-agent-platform/models/gemini/3-8-flash), [Developer's guide to Gemini 4 Neon](https://docs.cloud.google.com/gemini-enterprise-agent-platform/models/guides/gemini-4-neon)
- Fils du forum (rapports d'utilisateurs) : [Neon et Carbon en 403](https://discuss.ai.google.dev/t/india-billing-developer-program-credits-on-gemini-api-prepay-gemini-4-neon-carbon-403-on-vertex/202991), [logprobs coupés sur les Gemini 4](https://discuss.ai.google.dev/t/logprobs-turned-off-without-notice-for-gemini-4-hydrogen-argon-carbon-and-neon/203009)
