# Le classeur local dans le rappel — page de conception

*9 novembre 2026, session mémoire (96), à la demande de l'orchestration. Rien ne se code avant le
mot de Lucie. Sources : la page 03 du banc du 8 novembre (`docs/08-novembre-2026-banc/03-le-juge-local.md`),
le rappel des crochets de Claude Code (`src/claude_code_hook.rs`, lots 3 ter et 3 ter b,
`docs/07-novembre-2026-hooks-claude-code/08`), le décideur llama-server (`src/decider.rs`).*


**Actualisation du 12 novembre 2026 : proposition à lire par Lucie, sans activation.**
Les résultats Q8 sur card2 lèvent le préalable GGUF et la question de l'autorisation
GPU ; ils ne démontrent pas l'utilité du rappel réel. La recommandation est un
premier lot Q8/k5 dans le différé, deux fiches au maximum, évalué sur une référence
humaine. Les quatre choix qui demandent encore son mot sont regroupés au §8.
Le [ticket existant](../../../../docs/memoire/tickets/2026-11-11-trancher-et-verifier-le-classeur-local-du-rappel.md)
porte cette préparation. Aucun code, service, réglage utilisateur ni mémoire réelle
n'est modifié par cette actualisation ; les formes ci-dessous restent proposées.

## 1. Ce que les bancs ont établi

Banc historique du 8 novembre, conservé pour comparaison. Question de Lucie, mot pour mot : « ce bout de mémoire est-il pertinent avec le contexte de
l'agent ? ». Sur 88 positifs face à 4 négatifs chacun :

| juge | bonne fiche première | AUC dans le cas | temps pour 5 fiches |
|---|---|---|---|
| bge-m3 dense seul | 0,41 | — | (le rappel) |
| **Qwen3-4B-Instruct-2507**, préfixe partagé, fin de conversation 2 k | **0,76** | **0,92** | **≈ 1,0 s** |
| Qwen3-4B, forme courte | 0,73 | 0,89 | ≈ 0,45 s |
| Qwen2.5-1.5B, préfixe partagé | 0,65 | 0,84 | ≈ 0,42 s |
| Gemini 3.8 Flash, 5 en liste | 0,81 à 0,84 | — | 4,4 s p50, 11 à 23 s p95 |

C'est **un rangeur, pas un seuil** : l'échelle des scores bouge avec le contexte, on garde les
1 ou 2 premières. Les mesures de cette table sont en Python (torch ROCm, bf16) sur card0 de luciepc.
Ce placement est historique : les usages actuels vont sur card2, seat1 restant disponible
pour le jeu (§4). Les résultats du 12 novembre sont au §7 ; leurs temps ne sont pas
un speedup mesuré à protocole identique.

## 2. Ce qui est livré et ce qui reste proposé

- **Le rappel synchrone** (`UserPromptSubmit`) : `recall_fiches`, 3 candidates, filtre
  `state ∈ {current, to_review}`, sous `recall_timeout_ms` = 1 000 ms (13 à 70 ms mesurés en
  service dans un relevé ancien ; le relevé des 158 rappels du 9 novembre donne
  p50 161 ms, p95 397 ms). Ce qui est rendu tout de suite passe par la redite et le déjà-vu de la session.
- **Le rappel différé et son juge** (3 ter b) : un fils détaché (`hook --rappel-differe`) refait
  le rappel si le budget a coupé, puis **Gemini** juge les candidates (`juger`, sortie
  structurée, il ôte, jamais n'ajoute), au plus `recall_judge_max_per_hour` = 20 appels par
  heure. Le résultat est rangé et rendu au prochain outil ou au prochain tour. Ce juge ne voit
  **que le prompt** nettoyé, pas les tours d'avant.
- **La transcription** de la session est lisible par le crochet
  (`claude_code_transcript::tours_du_prompt`) : les tours, les appels d'outils, leurs résultats.
- **Le décideur llama-server** (`LlamaServerDecider`, la voie `RAG3WEAVER_BANC_DECIDE` du banc des
  fiches classées) : `/tokenize`, puis `/completion` d'un jeton avec `n_probs`, les
  probabilités des options normalisées. Il pose **`cache_prompt: false`** : chaque appel réencode
  tout, sans le préfixe partagé que le banc a mesuré.

### Trois chantiers distincts

- **M8, première tranche API livrée** (`8b3fab422`) : le plan de gabarits valide
  des contenus fournis sans application ni montée de la mémoire réelle. Il ne
  branche pas un modèle de classement et ne remplit pas la mémoire de Lucie.
- **M9, archiviste automatique du chat** : [conception et deux portes du rappel](../../../../docs/memoire/11-11-2026/05-archiviste-automatique-du-chat.md),
  sans raccordement livré. Décider de chercher, ranger et injecter sont des gestes
  distincts ; le classement local ne les remplace pas.
- **Cette proposition Qwen** : ordonner les candidates du rappel des crochets
  externes, avec contexte de conversation et budget borné. Elle n'est pas activée
  et n'impose pas ce modèle ou ce client au futur archiviste M9.

La clé `recall_classifier` proposée au §4 n'est pas acceptée par le parseur actuel
de `hooks.json`. Le client actuel a toujours `cache_prompt: false`, 40 probabilités
et un délai de 60 s ; le banc utilise un autre client de référence. Une fonction
existante ne prouve donc pas le raccordement proposé.

## 3. Où le brancher : recommandation à confirmer

**Commencer dans le fils du rappel différé, avec Q8/k5**, sans ajouter un second
juge à chaque appel. Si Qwen répond à temps, il prendrait la place de Gemini pour
ce passage ; en cas d'indisponibilité, le repli proposé du §6 resterait explicite.
Gemini servirait aussi de comparaison hors ligne, pas de second avis systématique
payant. Ce remplacement reste le choix nº1 du §8, pas une décision acquise.

Le temps médian mesuré du classement k5 est désormais 540 ms, pas 1 s. Cela
n'établit ni son impossibilité ni sa sûreté dans le synchrone : rappel, lecture du
contexte, tokenisation, file d'attente et éventuel démarrage à froid s'ajoutent.
Le banc n'a pas mesuré cette durée bout en bout. Garder le différé pour le premier
lot évite de modifier tout de suite le budget de la frontière utilisateur.
Le différé doit lui aussi avoir une échéance globale ; le rappel refait dispose
aujourd'hui d'un délai de 30 s, ce qui n'autorise pas une attente indéfinie du juge.

Proposition : cinq candidates et deux fiches **au maximum**, sans obligation d'en
injecter. Un rang n'est pas une preuve d'utilité et aucun seuil universel des
scores Qwen n'est validé. Préserver les refus, la redite et l'abstention ; un lot
ne doit pas convertir les cas sans fiche utile en deux injections forcées.
L'intérêt de passer le vrai rappel à dix candidates n'est pas mesuré : k10 ne
constitue ni un prérequis supplémentaire ni une amélioration déjà acquise.

## 4. La voie

- **Déclaration proposée dans `hooks.json`**, non disponible aujourd'hui :

      "recall_classifier": {
        "address": "http://127.0.0.1:7982",
        "model": "qwen3-4b-instruct-2507",
        "keep": 2,
        "candidates": 5,
        "context_tokens": 2000,
        "timeout_ms": 3000,
        "sync": false
      }

  Ce JSON illustre le contrat envisagé ; ne pas le copier dans une configuration
  utilisateur avant un lot autorisé et livré. Son absence conserve le rappel actuel.
- **Le client est `LlamaServerDecider`**, avec deux ajouts : `cache_prompt: true` et un même
  préfixe pour toutes les fiches d'un rappel (le contexte d'abord, puis la fiche, puis la
  question), pour que llama-server réutilise le cache de son slot d'une fiche à l'autre ; et un
  délai global borné par `timeout_ms` (aujourd'hui 60 s en dur). Reprendre aussi
  la normalisation des variantes oui/non et vérifier l'équivalence au client du banc,
  qui demandait 50 probabilités contre 40 dans le client actuel.
- **Le service tourne à luciepc**, jamais sur le portable (Lucie : pas de service lourd sur le
  portable). Le portable l'atteint par un tunnel ssh à côté de ceux des services d'embarquement
  (7979 à 7981), sur un port neuf.
- **Ressources déjà autorisées** : card2 de luciepc, PCI `0000:04:00.0`, pour
  poids, cache et calcul. La [consigne durable](../../../../docs/orchestration/11-11-2026/02-knowledge-dump.md)
  permet les modèles pertinents au travail autorisé sans redemander l'accord GPU
  à chaque usage. Coordonner capacité, VRAM et consommateurs réels ; les mesures
  comparatives gardent une fenêtre `poste mesure`. Pas de quota arbitraire de
  processus. Préserver seat1 ; le petit contexte stable de 40 KiB observé est toléré.
  L'audit de résidence a depuis arrêté le privé7882 et granite107/7880 ; ils ne sont
  pas des résidents à réserver pour ce projet. La reprise automatique à la demande
  reste à construire selon la [vision commune](../../../../docs/orchestration/visions/2026-11-12-residence-des-modeles-et-prechauffage.md).
  Cela ne constitue pas une autorisation de démarrer ou laisser résident un service
  dans ce travail documentaire.
- **Le modèle recommandé, non activé** : Qwen3-4B-Instruct-2507 en GGUF (Apache 2.0, licence relue sur la carte et le
  fichier LICENSE avant la pose).

## 5. Le contexte donné au juge

La **fin de conversation** proposée, inspirée du banc : les **deux derniers tours avant le
prompt, avec les résultats de leurs outils** (1 200 caractères chacun), puis le prompt, le tout
borné à 2 000 jetons réellement tokenisés et coupé par le début s'il déborde.
Cette borne est un contrat proposé : le jeu « début 2 k » du 12 novembre était
borné en caractères estimés ; sa première paire traite 2 214 jetons médians,
gabarit et fiche compris. Ne pas présenter ce banc comme la preuve de cette limite. Elle se lit dans la transcription au
moment du fils. Puis, pour chaque candidate : son nom, son `about`, et la question de Lucie.
Réponse : P(oui) contre P(non) au jeton suivant.

Le parcours Qwen proposé garde ce contexte sur luciepc. Un repli ou une
comparaison avec Gemini reste un appel distant selon les données autorisées pour
ce parcours ; ne pas promettre un traitement entièrement local tout en le gardant.

## 6. Sans llama-server

**Repli proposé, à valider avec le choix nº1.** `recall_classifier` absent, service injoignable ou
délai dépassé : le fils prend le juge Gemini s'il est disponible, sinon la politique « sans juge »
d'aujourd'hui (rien rendu pour des candidates venues du tour, la première au plein texte pour un
rappel refait). Une ligne au journal des crochets dit lequel a jugé et pourquoi. L'erreur et le choix de repli restent visibles au journal ; une erreur de service
ne devient pas une décision « aucune fiche utile ». L'attente est bornée et aucun
résultat tardif ne peut être compté comme livré au tour déjà commencé.

Un classeur qui ne fournit qu'un ordre ne sait pas, à lui seul, refuser toute la
liste. Le contrat d'abstention doit être précisé et vérifié dans le lot après
les choix de Lucie ; ni « garder deux » ni un seuil inventé sur ce banc ne le
résolvent. Le journal doit distinguer absence d'utilité, erreur et expiration. La cible de
3 s concerne la voie locale bout en bout ; le budget d'un repli Gemini est
à décider avec sa place au §8.1, puis à mesurer séparément. Ne pas additionner
des délais de repli et continuer à annoncer trois secondes pour le tout.

## 7. La mesure qui dira si ça vaut le coût

Après décision et autorisation du lot : préparer un rejeu isolé de tours représentatifs
et figés, avec les mêmes candidates et le même contexte pour les trois voies : rappel
actuel, Gemini et Qwen. `hook --rejouer` n'est pas une garantie suffisante à lui seul :
prouver aussi l'isolation des journaux, configurations et écritures mémoire avant
la mesure. Aucun de ces rejeux n'est autorisé ni lancé par la présente page.

- **La précision des fiches glissées** : pour chaque fiche rendue, est-elle utile au tour ? La
  référence recommandée : Lucie sur 50 tours, avec cas utiles et cas sans fiche utile.
  Gemini peut fournir une annotation secondaire, explicitement faillible (il a construit
  les positifs du banc historique) ; voir le choix nº4.
- **Le temps par tour** : p50 et p95 du fils (le rappel, le contexte, le classeur), et du tour
  synchrone si l'option `sync` est essayée.
- **La VRAM de luciepc** : la carte du service, avant et après la pose, sous le jeu de Lucie.
  Le banc Q8 du 12 novembre mesure une hausse de 6,655 Gio sur card2 avec sa recette,
  différente de l'estimation ancienne de 5 Go. Relever la capacité disponible et
  les consommateurs au moment du lot ; ce n'est plus un accord sur card0 à attendre.
- **L'écart du GGUF** : le banc était en torch bf16. Avant la mesure réelle, le jeu du banc
  (88 cas) a été rejoué et le préalable historique 0,74 est franchi en k5 ; ne pas
  demander un troisième banc identique pour ouvrir le choix produit.
- **Critères proposés, non adoptés** : sur la référence retenue, comparer précision,
  nombre de rappels utiles et abstentions aux mêmes tours du rappel actuel et de Gemini ;
  ne pas gagner des fiches utiles au prix d'injections systématiquement inutiles.
  Cible initiale : pas de dégradation observée de précision et p95 bout en bout de la
  voie locale sous 3 s, ressources compatibles avec la charge réelle. L'ancien « Gemini
  moins 5 points » n'est pas une décision de Lucie et n'est pas un seuil acquis.

### L'écart du GGUF, mesuré (session banc, 11 novembre 2026)

Le préalable ci-dessus est levé : `docs/banc/11-11-2026/03-le-juge-en-gguf.md` (master
96a98a452). Qwen3-4B-Instruct-2507 converti en GGUF par le llama.cpp de Lucie, servi par
llama-server sur card0 de luciepc, même jeu de 440 paires, oui/non par les logits du premier jeton.

| forme | bonne première | AUC | par cas de 5 |
|---|---|---|---|
| bf16 torch (8 novembre), préfixe partagé, 2 k | 0,76 | 0,92 | 1 017 ms |
| **Q8_0, `cache_prompt`, 2 k** | **0,76** | **0,92** | **1 076 ms** |
| Q8_0, forme courte | 0,77 | — | 445 ms |
| Q8_0 sans `cache_prompt` | — | — | 3 288 ms |
| Q4_K_M, 2 k | 0,76 | — | pas plus rapide sur card0 |

La même première fiche que bf16 dans 97 % des cas. **Q8_0 avec `cache_prompt` est le choix**,
les cinq fiches d'un rappel à la suite sur le même slot, le contexte d'abord, la fiche à la fin :
sans le cache, trois fois plus lent. Le client de référence est `~/.cache/rag3db-banc-notes/q_llama.py`
à luciepc (gabarit appliqué côté client, `/completion` avec `n_predict` 1 et `n_probs` 50, score =
sigmoïde(log P(oui, Oui) − log P(non, Non))). `LlamaServerDecider` doit donc prendre
`cache_prompt: true` (§ 4) et ce score.

La page 04 (la précision du rappel d'aujourd'hui) reste la condition de fond : le classeur ne
vaut sa seconde que lorsque la mémoire de Lucie a du contenu à ranger.

### Q8/k5 et k10 sur card2, mesurés le 12 novembre

[Rapport du banc et preuves](../../../../docs/banc/12-11-2026/01-le-juge-sur-card2.md),
master `ee40dbcd6`, ticket de mesure clos. 88 cas construits, chacun avec un positif.

| candidates | utile première | utile dans les deux premières | médiane par cas | p95 par cas |
|---|---:|---:|---:|---:|
| k5 | 67/88 (76,1 %) | 84/88 (95,5 %) | 539,9 ms | 635,8 ms |
| k10 | 58/88 (65,9 %) | 78/88 (88,6 %) | 786,6 ms | 883,4 ms |

Sur les 34 cas de test réservés : première utile 26/34 puis 24/34, dans les deux
premières 32/34 puis 30/34. Les 440 scores communs sont identiques : les cinq
nouveaux distracteurs modifient les rangs, pas ces scores. Ils sont tirés hors
session et ne remplacent pas une mesure de couverture d'un vrai rappel k10.
Le corpus force un positif par cas : il ne prouve ni la précision des injections
réelles ni l'abstention lorsqu'aucune fiche n'est utile.

Recette : Q8_0, llama.cpp figé, un slot, préfixe partagé, cache RAM global à zéro,
effacement du slot avant chaque série et pas de warmup demandé. Les formes de
contexte sont estimées en caractères. K10 suit k5 sur le même serveur ; ces temps
n'incluent ni le rappel, ni son contexte, ni le démarrage serveur. Les différences
avec le banc historique interdisent d'en déduire un speedup à configuration égale.
Poids/KV/calcul sur card2, petit contexte seat1 stable 40 KiB VRAM/2 080 KiB GTT ;
ce relevé ne garantit pas les FPS et les compteurs moteur HIP restent inconnus.

La [mesure du rappel du 9 novembre](04-la-precision-du-rappel-d-aujourd-hui.md)
reste une limite de fond : hors fiche propre, quatre candidates pertinentes sur
104 rappels de cette session historique. Aucun reclassement ne crée une candidate
utile absente. Ce n'est ni la preuve que le contenu actuel est inchangé, ni une
raison d'attendre l'activation de M9 pour poser les décisions de cette page.

## 8. Questions pour Lucie

**Déjà acquis, à ne pas redemander** : card2 pour le travail pertinent, disponibilité
de seat1 à préserver, Q8_0 avec préfixe partagé qualifié sur le banc, k5/k10 mesurés.
Le choix proposé k5 conserve la référence testée ; passer à dix candidates ne
promet pas un meilleur rappel réel. M8 API livrée, M9 et adoption réelle distincts.
Ces acquis ne tranchent pas les quatre choix de produit suivants.

1. **Quelle place pour Gemini ?** Recommandation : Qwen remplace le juge du
   différé lorsqu'il répond à temps ; Gemini reste comparaison hors ligne et
   repli explicite en cas d'échec, pas second avis à chaque tour. Alternative :
   Qwen classe puis Gemini confirme systématiquement, avec un appel distant
   supplémentaire. Choisir la place et le repli ensemble, sans les adopter par défaut.
2. **Combien de fiches au maximum : une ou deux ?** Recommandation : deux au
   maximum pour le premier essai, en comparant aussi le résultat à une sur les
   mêmes tours. Le gain 84/88 contre 67/88 à k5 porte sur la présence du positif
   dans un jeu construit ; il ne mesure pas la précision des deux injections.
   Zéro reste une sortie normale : ce plafond n'est jamais un quota.
3. **Différé d'abord ou essai synchrone dès le premier lot ?** Recommandation :
   différé uniquement au départ, échéance de 3 s proposée pour la voie locale
   bout en bout et repli nommé avec son propre budget explicite.
   Mesurer le bout en bout avant un essai synchrone optionnel dans la seconde
   actuelle. Les 540 ms du banc seul ne prouvent pas qu'il y tienne sous charge
   ou à froid. Alternative : autoriser les deux voies dès la mesure initiale.
4. **Qui fait référence sur les 50 tours ?** Recommandation : Lucie, sur un
   échantillon comprenant des cas utiles et des cas sans fiche utile, figé avant
   comparaison ; Gemini peut annoter en second sans devenir vérité automatique.
   Alternative : Gemini seul comme référence provisoire, en gardant explicitement
   le biais des positifs construits et en bornant la portée de toute conclusion.

Répondre à ces quatre questions fixe le périmètre à proposer pour le lot ; cela
n'autorise pas implicitement son activation dans la mémoire réelle. Le niveau de
décision reste celui du [§2 bis de l'orchestration](../../../../docs/orchestration/11-11-2026/03-reprise-par-codex.md#2-bis-comment-se-prennent-les-décisions).
Aucun nouveau plafond de VRAM, accord card0 ou banc GGUF déjà fait n'est à ajouter
aux conditions bloquantes. Le critère d'utilité réelle reste à prouver par le lot.

## 9. L'ordre proposé après son mot

1. Consigner les réponses du §8 et le périmètre du lot, avec critères de mesure
   et repli ; relire les primitives client/cache/budget disponibles à cette date.
   Réutiliser les bancs acquis, sans rejouer k5/k10 à recette identique par habitude.
2. Sous autorisation de code distincte : raccorder le client au contrat du banc,
   la déclaration et le différé ; témoins sur faux serveur pour budget global,
   cache, repli visible, zéro fiche, redite et contexte. Pas d'activation utilisateur.
3. Préparer le rejeu isolé et la référence choisie, comparer classement et utilité
   réelle, puis publier précision, couverture, abstention et p50/p95 bout en bout.
   Coordonner card2 selon capacité ; une mesure comparative conserve sa fenêtre.
4. Soumettre le résultat à Lucie. Une activation réelle, ou une voie synchrone
   retenue ensuite, demande sa décision explicite et sa validation propre.
