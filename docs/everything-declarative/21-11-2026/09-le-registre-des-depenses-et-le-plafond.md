# Le registre des dépenses et le plafond

21 novembre 2026, session everything declarative. Lucie, vers 19 h 50 : « monitorer l'usage et ne jamais dépenser plus
de 200 par mois ; un moteur de facturation : volume × coût pour chaque slug ». Puis vers 20 h : « pour brancher Claude,
on veut d'abord savoir gérer ces coûts ».

> **Ce registre et ce plafond sont la condition d'entrée de tout fournisseur payant neuf.** Aucun modèle payant de plus
> (Claude compris) ne se branche avant que ce lot ait atterri.

Le plafond est de **200 USD par mois**. C'est le défaut de la config : `"currency": "USD"`, `"monthly_cap": 200`.

## Ce qui existe déjà

- **`connect_llm`** (`model_source.rs`) construit tout modèle déclaré dans `models.llm`, pour tous les appelants :
  l'équipier, l'archiviste, les juges, le chat, le survivant. C'est le point unique où le registre s'enroule.
- **`meter.rs`**, le compteur, a déjà la bonne doctrine : **on enregistre des faits, jamais un prix**. Les tarifs
  changent, et un prix rangé à côté d'un appel est un verdict qui survit à ses raisons. Le prix est une table, appliquée
  à la lecture. Mais le compteur vit en mémoire, le temps d'une session, et seul l'agent l'alimente.
- **`Usage`** (`llm.rs`) porte `prompt_tokens`, `cached_prompt_tokens`, `completion_tokens` et `reasoning_tokens`.
  `cache_reported` distingue un compte de cache absent d'un compte nul. **Il manque l'écriture du cache** : le client
  Anthropic additionne `cache_creation_input_tokens` dans `prompt_tokens`, alors que ces jetons coûtent 1,25 fois
  l'entrée. Le lot ajoute `cache_write_tokens`, rempli par chaque client qui le reçoit.

## 1. Le registre

**Où** : `~/.local/state/rag3weaver/usage/<aaaa-mm>.jsonl`, un fichier par mois.

**Qui écrit** : un enrobage `MeteredLlm` posé par `connect_llm` autour de chaque client, et par lui seul. Aucun appelant
ne peut l'oublier. Deux témoins le tiennent :
- tout `Arc<dyn Llm>` rendu par `connect_llm` est enrobé ;
- **une garde de source** : hors des tests, aucun fichier ne construit un client payant (`OpenAiLlm::new`,
  `AnthropicLlm::new`, `vertex_with_source`) ailleurs que dans `model_source.rs`. Un appel direct qui contournerait le
  registre rougit.

**Une ligne par appel**, écrite quand la réponse arrive :

```json
{"t":"2026-11-21T19:52:03Z","id":"u-…","kind":"settle","slug":"gemini-3.5-flash","provider":"vertex",
 "input":1200,"cache_write":0,"cache_read":8000,"output":340,"reasoning":120,"mode":"direct",
 "caller":{"agent":"agent:…","session":"…","run":"…","by":"human|agent|host"}}
```

- **Les jetons viennent de `usage`**, jamais d'une estimation. Une réponse sans `usage` s'écrit `"tokens":"unknown"`, et
  elle compte à sa borne haute (voir la réservation).
- **Les scripts du banc** écrivent par la même porte, une sous-commande `rag3weaver-backend usage record …`, pas par un
  fichier écrit à la main.

**Plusieurs écrivains** : l'hôte, plusieurs clients et les scripts du banc écrivent le même fichier, depuis des processus
différents.
- Une ligne s'ajoute d'un seul `write` en `O_APPEND`. Sa taille est bornée sous `PIPE_BUF` (4 Kio), donc deux lignes ne
  se mêlent jamais.
- Le cumul se lit et la réservation s'écrit **sous un verrou de fichier** (`flock` sur `usage/<aaaa-mm>.lock`).

## 2. Les prix

**Où** : `~/.config/rag3weaver/prices.json`.

```json
{
  "currency": "USD",
  "monthly_cap": 200,
  "alerts": [0.5, 0.8],
  "prices": {
    "gemini-3.5-flash": [
      {"from": "2026-09-01", "input": 0.30, "cache_write": 0.30, "cache_read": 0.03, "output": 2.50, "batch": 0.5}
    ]
  }
}
```

- Les prix sont **par million de jetons**, dans la monnaie de `currency`. Un tarif se choisit **à la date de l'appel**
  (`from` le plus récent qui la précède) : le changement d'Argon au 1er décembre s'écrit comme une seconde entrée, sans
  réécrire l'historique.
- `reasoning` se compte au prix de `output`, sauf si le tarif dit autrement.
- **Un slug sans prix** compte « prix inconnu ». Il n'est jamais compté à zéro : son appel est refusé comme un
  dépassement, en le disant.
- **Un plafond dans une autre monnaie que les prix** est refusé au chargement. Il n'y a jamais de conversion implicite.
- **Un fichier illisible** refuse tout appel payant, en le disant. Le défaut n'est pas « aucun plafond ».

## 3. Le plafond

**Il s'applique avant l'appel, et il est réservé.** Contrôler le cumul sans réserver a une course : deux appels lisent le
même cumul et passent tous les deux. Sous le verrou :

1. le cumul du mois est lu ;
2. la **borne haute** de l'appel est calculée : l'entrée qu'il envoie, au prix plein sans cache, plus son `max_tokens`
   au prix de sortie ;
3. si cumul + borne dépasse le plafond, l'appel est refusé ;
4. sinon une ligne `reserve` est inscrite, avec la borne et un `id`.

Quand la réponse arrive, une ligne `settle` avec le même `id` remplace la réservation par le vrai coût. **Une
réservation jamais soldée** (un processus mort, un appel coupé) **compte à sa borne haute** : c'est le côté sûr.

**Le témoin** : deux appels concurrents au bord du plafond, de deux processus. Un seul passe.

**Au-delà de 100 %**, tout appel payant **non humain** est refusé : celui d'un agent, d'un juge, de l'archiviste, du
survivant. Le refus dit le cumul, le plafond et le mois. Un appel qu'un humain lance lui-même (un tour de chat que l'hôte
a signé `by.kind == "human"`) passe, et il est compté. Le plafond protège de la dépense d'une boucle, pas de celle de
Lucie.

**Les alertes à 50 % et 80 %** vont dans « À intervenir », une seule fois par seuil et par mois.

**La projection du mois** est linéaire : le cumul divisé par les jours écoulés, multiplié par les jours du mois. Elle
alerte aussi, quand elle passe le plafond avant le 80 % réel.

## 4. La vue

- **Un outil MCP `usage_report`**, en lecture : le mois en cours ou un mois donné, par slug, par jour, par appelant.
  Il rend aussi le cumul, le plafond, la projection, les réservations ouvertes et les slugs sans prix.
- **Une ligne dans `daemon_status`** : `dépenses : 41,20 / 200 USD (21 %), projection 59 USD`.
- **Le relevé quotidien de G**, s'il le veut, lit `usage_report`.

## Hors de portée des agents

- `prices.json` est sous `~/.config/rag3weaver`, que le plancher des dossiers protégés tient en code.
- **Le registre** est sous le dossier d'état de l'hôte, entré au plancher par le lot d'arbre `arbre-anthropic-folder`,
  déposé le 21 au soir, avec son témoin `the_hosts_state_folder_is_protected`.
- **Le lot `v1-defaults`** ajoute qu'un dossier protégé s'ouvre pour une seule commande, sur un oui humain, et jamais par
  un accord durable. Un agent ne garde donc jamais le droit de réécrire sa dépense ou de mettre un prix à zéro.

Le lot de la facturation **dépend des deux lots d'arbre** : il se rejoue quand ils ont atterri.

## Les témoins du lot

| Témoin | Ce qu'il prouve |
|---|---|
| une ligne par appel, ses jetons pris de `usage` | le registre écrit ce que le fournisseur rapporte, cache écrit et lu séparés |
| une réponse sans `usage` | notée « inconnue », comptée à sa borne haute, jamais à zéro |
| deux processus au bord du plafond | un seul passe (la réservation sous verrou) |
| une réservation jamais soldée | comptée à sa borne haute |
| un slug sans prix ; un plafond dans une autre monnaie ; un fichier de prix illisible | refusés, en le disant |
| un tarif qui change au 1er décembre | un appel du 30 novembre et un du 1er décembre se comptent chacun à son tarif |
| au-delà du plafond | un appel d'agent est refusé, un appel humain passe et se compte |
| la garde de source | aucun client payant construit hors de `connect_llm` |

## Ce que ce lot ne fait pas

- **Il ne remplace pas la facture du fournisseur.** Comme le dit `meter.rs`, un compteur local est une mesure, jamais une
  autorité. Le registre dit ce que le fournisseur a rapporté. Un relevé mensuel qui comparerait les deux serait un lot à
  part.
- **Il ne compte pas les modèles locaux** dans le plafond : un `compatible` à une adresse locale n'a pas de prix. Il
  s'écrit au registre avec `"provider":"local"`, pour le volume, et compte zéro **parce que la config le dit**
  (`"local": {"price": 0}`), pas par défaut.
