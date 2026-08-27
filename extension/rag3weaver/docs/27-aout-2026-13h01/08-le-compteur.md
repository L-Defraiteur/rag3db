# 08 — Le compteur : mesurer avant de facturer

Demande de Lucie, 27 août : *« le coût en tokens ? on fait un début de
processeur de crédit même si après forcément ce sera pas chez la machine du
produit de celui qui code ? »* — puis, en précisant : *« le compteur oui
important par contre, autant pour tts/stt et llm, même juste pour savoir sur sa
machine locale ou provider ce qu'on a utilisé exactement »*.

C'est cette seconde formulation qui a décidé le design.

## 1. Compteur, pas crédit — et la distinction n'est pas de la prudence

Un **crédit** est un solde : dotation moins consommation. Pour valoir quelque
chose il doit être **autoritatif**, donc vivre là où personne ne peut le
réécrire — pas sur le poste de celui qui code. Lucie l'a dit elle-même, et
c'est ce qui tranche : on ne le fait pas ici, et pas maintenant.

Un **compteur** est un relevé de faits. Il peut vivre partout, il ne prétend
rien, et il est utile dès le premier appel.

> **Un compteur local est une mesure, jamais une autorité.** La facturation du
> fournisseur est la vérité ; nous disons ce qu'on a demandé et ce qu'il a
> rapporté.

## 2. L'unité n'est pas le jeton

C'est le point que la remarque sur le TTS et le STT a fait apparaître, et il
change la forme du module.

Un compteur qui ne connaîtrait que les jetons ne pourrait pas mesurer une
synthèse vocale — facturée au caractère ou à la seconde d'audio. Il faudrait un
second compteur, et **deux compteurs ne se totalisent jamais**.

D'où la primitive : `(ressource, unité, quantité)`. La même forme pour un LLM
distant, un LLM local, un TTS, un STT.

```
llm.gemini-3.5-flash   1 200 input_tokens, 8 000 cached_input_tokens, 300 output_tokens
tts.piper.fr           1 840 characters
stt.whisper-large         73 audio_seconds
```

**Un appel, une ligne, plusieurs unités.** Un enregistrement par unité
dédoublerait l'attribution et rendrait les totaux mal recomposables.

## 3. Le slug est le joint vers plus tard

Lucie : *« le pattern de crédit devra pouvoir servir à des gens qui vendent des
produits divers, faut peut-être des slugs setupables […] mais oui pas tout de
suite, ça veut rien dire »*.

Elle a raison sur les deux moitiés, et elles ne s'opposent pas. Le crédit ne
veut rien dire aujourd'hui ; **le slug, si**, et il coûte trois fois rien.

Le compteur enregistre `llm.gemini-3.5-flash`, `tts.piper.fr`. Le jour où
quelqu'un vend quelque chose, sa table résout des slugs vers des prix — **une
table à écrire, pas chaque point d'émission à retoucher**.

C'est la même règle que pour l'attribution : *ce qu'on ne peut pas rattraper
entre maintenant, le reste attendra.* Et le slug est **paramétrable**
(`Agent::with_resource`), parce que le nom d'un modèle n'est pas le nom du
produit qu'on vend.

## 4. Des faits, jamais un prix

Règle reprise telle quelle du [doc 05 §2.1](05-la-reputation-des-abstractions.md) :
les tarifs changent, et **un prix rangé à côté d'un appel est un verdict qui
survit à ses raisons**. La tarification est une table remplaçable, appliquée au
moment de lire.

## 5. Le cache est une unité à part, et c'était un trou

`read_sse` lisait `prompt_tokens` et `completion_tokens` et **jetait**
`prompt_tokens_details.cached_tokens`, que le fournisseur envoie pourtant. Or
l'entrée servie depuis le cache coûte environ **dix fois moins**.

Les confondre fausse un coût d'un ordre de grandeur — **dans le sens qui
flatte**, donc dans le sens qu'on ne va pas vérifier.

Deux précautions valent d'être notées :

- **La part en cache est comprise dans `prompt_tokens`**, pas en plus : c'est
  ainsi que les API compatibles OpenAI la rapportent. `Usage` garde donc le
  chiffre du fournisseur tel quel — diverger de son total serait pire que ne
  rien mesurer — et c'est **au moment d'enregistrer** qu'on retranche le cache
  de l'entrée plein tarif, sinon on compterait deux fois ce qui n'a été payé
  qu'une, au tarif fort.
- **`0` quand le fournisseur ne dit rien** : on ne devine pas un cache.

## 6. Pourquoi il fallait aussi `model`

Sans le nom du modèle, aucun coût n'est calculable. `OpenAiLlm` avait le champ,
`Llm::name()` existait, et ni l'un ni l'autre n'atteignait l'événement — un
champ qu'on ne peut pas reconstituer après coup.

## 7. `Consumed` est distinct de `LlmCall`, et c'est délibéré

Le premier dit **ce qui a été consommé**, le second **comment la boucle s'est
passée** — itération, raison d'arrêt, réessais. Un TTS émettra le premier et
jamais le second ; les mélanger obligerait la voix à se déguiser en appel de
modèle.

La redondance des deux entiers est assumée. Confondre un diagnostic et un
relevé de consommation ne l'aurait pas été.

`Consumed` part sur le sujet `agent`, donc les graphes de trace l'enregistrent
**par défaut**. Un sujet auquel personne ne s'abonne serait le mécanisme
décoratif qu'on passe la journée à débusquer : la question « ce run a coûté
quoi » doit avoir sa réponse sans configuration préalable.

## 8. Le défaut que ça a fait sortir

`Consumed` portait d'abord run, agent, ressource et unités — et rien qui
distingue deux appels du même modèle dans le même run. Or `Trace` **n'a pas de
`hashsafe`** : son uuid dérive de tous ses champs.

Résultat : trois consommations identiques ont fusionné en une ligne. Le graphe
annonçait **18 enregistrements, le catalogue en contenait 16**, et rien nulle
part ne disait où étaient passés les deux autres.

Deux corrections, et la seconde est la vraie :

- `Consumed` porte son **itération** — ce qui le distingue, et ce qu'on veut
  vraiment savoir : quel tour a coûté quoi ;
- **le traceur sait dire ce qui a été consommé** au lieu du mot seul. Il tombait
  dans le cas générique — résumé « Consumed », détail vide.

**Dette nommée** : `Trace` sans `hashsafe` fusionnera d'autres doublons. Ce
n'est pas propre à cet événement — n'importe quels deux événements identiques
disparaissent l'un dans l'autre, en silence.

Le test qui l'a attrapé compare **ce que le graphe dit avoir écrit à ce qui
existe**. C'est le genre d'assertion qu'on croit redondante.

## 9. Ce qu'il ne fait pas

- **Il ne remonte pas l'arbre des runs.** Un parent coûte ce qu'il a consommé
  plus ses enfants, et cet arbre vit dans le graphe (`CHILD_OF`) : le total
  d'une branche est un parcours. Le faire en mémoire serait deviner l'arbre au
  lieu de le lire.
- **Personne ne lit encore le relevé.** `Meter::describe()` existe et rien ne
  l'affiche — ma propre décoration, et elle est notée comme telle.

## 10. Et pourquoi pas tiktoken

Question posée, et la réponse est non, pour une raison de fond.

Les fournisseurs distants **rapportent le compte exact** dans la réponse. Un
compte local serait une estimation qui *contredit la facture* ; quand une
autorité existe, on prend son chiffre.

Et tiktoken est le BPE d'OpenAI : il ne correspond ni au tokenizer de Gemini ni
à celui de Claude. S'en servir pour prédire un coût Vertex donnerait un nombre
précis et faux — le mode d'échec exact contre lequel tout ce dépôt se prémunit.

Là où un compte local sert : les **modèles locaux**, où personne ne facture mais
où l'on veut savoir — et le tokenizer y est déjà. Et un contrôle de budget
*avant* d'envoyer, où `caractères / 4` **déclaré comme estimation** est honnête,
ou l'endpoint `countTokens` du fournisseur si la précision compte. Aucune
dépendance nouvelle dans les deux cas.

## 11. Ce que ça va permettre de vérifier

Le facteur 24 de l'[absorption](04-la-session-tient-l-invite.md) est mesuré **en
caractères envoyés**, pas en jetons facturés. Contre un fournisseur qui met les
invites en cache, les deux peuvent diverger : réécrire un vieux résultat change
le préfixe, et un cache de préfixe s'arrête au premier jeton modifié.

Le design semble tomber bien — `Stale` réécrit dans une fenêtre qui glisse près
de la **fin** de l'historique, la tête restant stable, et la réécriture est
idempotente donc chaque résultat n'invalide qu'une fois. **Mais ce n'est qu'une
explication convaincante**, et seul `cached_input_tokens` le dira.
