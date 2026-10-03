# Comment on écrit chez tracel-ai : étude avant d'envoyer nos correctifs

**3 octobre 2026, session « optimiseur ».** Lucie est d'accord pour proposer
nos correctifs à tracel-ai, à une condition de ton : ne pas paraître
mécanique, et regarder d'abord comment on écrit là-bas. Cette page est cette
lecture. Elle s'est faite par l'API de GitHub en lecture seule sur leurs trois
dépôts publics (burn, cubek, cubecl) : **rien n'a été écrit chez eux, rien
n'est parti.** Les textes réécrits sont dans
[02](02-les-textes-reecrits.md) ; les textes d'avant restent dans
[le 01 du 2 octobre](../2-octobre-2026-00h15/01-les-sept-pr-amont.md).

Échantillon : les 300 derniers envois (issues et PR, hors robots) de chaque
dépôt, soit trois semaines de burn et deux mois de cubek et de cubecl ; une
centaine lus en entier, avec leurs commentaires et leurs revues.

## 1. Leur position sur l'aide d'une IA

**Burn l'autorise par écrit et n'exige aucune déclaration. Ce qu'il exige,
c'est que l'autrice comprenne chaque ligne et la défende elle-même.** cubek
n'a pas de guide de contribution ; celui de cubecl n'en dit rien.

Le texte (`CONTRIBUTING.md` de burn, ajouté par burn#4569 en mars 2026) :

> Using LLMs and AI tools to generate code that is part of a contribution is
> allowed. […] You are the author, not your AI tool. […] Read and understand
> every line before submitting. […] Be prepared to explain the rationale
> behind any change during review. Do not use "AI generated" as a
> justification for low-quality code.

> PR authors must understand, justify, and explain every change they propose.
> […] The origin of the code doesn't matter; what matters is that you own it
> intellectually and can stand behind it during review.

Aucune case à cocher, aucune ligne à ajouter : le gabarit de PR n'en parle pas.

La pratique, lue dans leurs fils :

| ce qui se passe | où | ce qu'ils en disent |
|---|---|---|
| une déclaration volontaire | burn#4231 | laggui : « Using coding tools isn't against the rules, and as a reviewer this already gives me helpful context […] I do appreciate it » |
| des PR d'extérieurs avec une ligne de déclaration | burn#5663, #5866, #5592, #5574 | fusionnées en un à quatre jours, sans remarque |
| des réponses de revue collées depuis un agent | burn#3383 | antimora : « What are your own thoughts? I feel I am chatting with an AI agent :-) […] I am all for AI-assisted coding, but this is not the right approach. » Fermée. |
| une autrice qui ne fait que relayer | burn#4569 | antimora : « the developer was basically just a conduit between the maintainers and the agent » ; l'autrice doit posséder « every aspect of the PR, including the description and driving it to completion » |
| un envoi que l'auteur ne comprend pas | burn#4567 | antimora : « pure AI generated blind submissions. Use the AI tools but you must understand and control. » Fermée le jour même. |
| un gros rapport brut | burn#5328 | laggui : « Prefer small, scoped groups of validated findings, ideally with a reproducer or failing test » |
| des commentaires de code laissés par l'outil | cubecl#1389 | ThierryCantin-Demers : « Seems to be leftover AI comments, would be better if removed » |

Ce que cela veut dire pour nous, sans trancher à la place de Lucie :

- Notre règle « aucune mention d'IA » ne contourne aucune règle de chez eux.
- Leur règle réelle engage Lucie en personne : c'est elle qui répondra en
  revue, avec ses mots. Les correctifs sont petits ; il faut qu'elle les ait
  lus et compris avant l'envoi (une fiche par correctif est en fin du 02).
- Déclarer ou non était à elle de décider : se taire est permis, une ligne
  sobre du genre de celle de burn#5663 (« I made this change with help from
  Claude Code. I reviewed it and can explain it. ») est chez eux plutôt bien
  vue. **Lucie a tranché le 3 octobre : on déclare**, dans chaque envoi. La
  ligne ne doit affirmer que ce qui est vrai le jour de l'envoi : elle ne dira
  « I reviewed it and can explain it » que si Lucie a relu les correctifs.
- Les réponses de revue ne peuvent pas être rédigées par moi puis collées :
  c'est exactement ce qu'ils appellent « conduit ». Je peux aider à comprendre
  et à vérifier ; la formulation doit être la sienne.

## 2. Ce que font les extérieurs dont les PR passent

| | burn | cubek | cubecl |
|---|---|---|---|
| corps d'une PR (médiane) | 27 lignes | 21 lignes | 18 lignes |
| première réponse d'un mainteneur | 15 h (les petits correctifs dans la journée) | environ 7 jours, souvent une approbation sans un mot | 3 jours |
| qui relit | laggui surtout, antimora | louisfd | nathanielsimard, ThierryCantin-Demers, wingertge |
| forme | les titres du gabarit, gardés par deux PR sur trois | titres maison (`Problem / Fix / Tests`) | titres maison, sortie d'erreur collée |

- **Le titre** est un commit conventionnel : `fix(nn): …`, `fix(cuda): …`. Les
  nôtres le sont déjà.
- **Le ton** est déclaratif et technique, sans salut ni excuse. La première
  phrase dit ce qui casse pour l'utilisateur : « Anyone calling `Tensor::roll`
  […] with a positive shift gets elements shifted towards lower indices
  instead of higher ones » (burn#5912). Puis la cause, en nommant la fonction.
  Puis le correctif en une ou deux phrases.
- **Les tests** : un test de régression nommé, la phrase « it fails on `main`
  and passes with this change » (burn#5663), et la liste exacte des commandes
  lancées avec leur résultat. Ce que demande le plus souvent un mainteneur en
  retour, c'est un test de plus (burn#5868, #5911, #5801).
- **Les mainteneurs** répondent court et chaleureux : « Good catch, thanks for
  fixing this discrepancy. » (laggui, burn#5912). Ils écrivent eux-mêmes plus
  court que les extérieurs (16 lignes de médiane).
- **Les cases du gabarit ne sont pas exigées, la franchise si.** Sur 56 PR
  d'extérieurs fusionnées dans burn, 19 cochent `cargo run-checks`, 19 la
  laissent décochée, 18 n'ont pas de case. Ce qui passe bien : dire ce qu'on a
  lancé. burn#5663 : « I ran the burn-flex and Flex backend-test subsets
  listed under Testing, not the full `run-checks`. » cubecl#1468, fusionnée
  les huit cases vides : « Not done — I'd rather leave the boxes empty than
  tick them. »
- **L'issue préalable n'est pas exigée pour un petit correctif** (10 PR
  fusionnées sur 56 n'en ont aucune ; burn#5912 écrit « None found; searched
  open and closed issues/PRs for … »). Quand il y en a une, le même auteur
  l'ouvre quelques minutes avant la PR : c'est une fiche de bug, pas une
  discussion.
- **« Validate your PR with burn »** (gabarit de cubek et de cubecl) : fait à
  la lettre par les grosses PR seulement. Pour une petite, ce qui est apprécié
  à la place est un tableau avant/après de la suite de burn lancée en local
  avec la révision corrigée (cubecl#1671 : « 1512 passed, 6 failed » puis
  « 1518 passed, 0 failed » ; cubek#772).
- **Un compte nouveau** attend qu'un mainteneur approuve les workflows de CI ;
  relancer poliment au bout d'une semaine est admis (cubek#648, cubecl#1717).

## 3. Trois acceptées, trois fermées

Acceptées (burn) :

- **#5912** `fix(tensor): roll shift direction to match torch.roll`, +32 −6,
  compte sans historique, pas d'issue, fusionnée en six heures. Elle explique
  pourquoi les tests existants ne voyaient rien et ajoute un test qui ne peut
  pas passer par hasard.
- **#5837** `fix(autodiff): correct repeat_dim gradient ordering`, +25 −2,
  dix-neuf lignes de texte, fusionnée en dix heures.
- **#5925** `fix(tch): flatten the broadcast alpha…`, +31 −1, issue ouverte
  seize minutes avant, fusionnée en quatorze heures.

Fermées :

- **burn#5596** : correctif au mauvais niveau. « The race in #5573 is valid,
  but I think we need to address this differently. »
- **cubecl#1510** : refus de principe, un repli après un échec d'autotune.
  nathanielsimard : « We can't have a fallback when we fail to run an
  autotune, since we must never clone the input of an autotune run ».
- **cubek#700** : déjà contourné côté burn. L'auteur a répondu par une
  reproduction sur `main` et rouvert plus étroit (#772).

Les refus ont cinq causes : doublon d'un travail de mainteneur en cours,
correctif au mauvais niveau, backend déprécié, test redondant, PR cosmétique.
Aucune ne tient à la forme du texte.

## 4. Ce que cette lecture change à notre plan

1. **Un fait nouveau sur la PR 1.** burn affirme dans un test que Flex32 n'est
   *pas* supporté sur Vulkan (`should_support_vulkan_dtypes`, dans
   `crates/burn-wgpu/src/lib.rs`, écrit par laggui le 1er octobre dans
   burn#5921). cubecl, lui, génère ses tests SPIR-V pour `flex32`
   (`tests_spirv` de `cubecl-wgpu`) sans l'inscrire. Notre ligne peut donc être
   un oubli comme un choix : la PR 1 doit **poser la question** en première
   phrase, et dire que l'assertion de burn sera à retourner.
2. **La PR 5 (flash attention) ne part pas maintenant.** Elle ne se teste
   qu'avec Flex32 inscrit sur Vulkan (PR 1) et le masque corrigé (PR 6) ; et
   son principe, un repli par conversion côté burn, ressemble à ce que
   cubecl#1510 s'est vu refuser. Elle devient une question dans l'issue burn.
3. **Deux précédents à citer.** burn#3551 de wingertge, « [Fix] Add some
   missing handling for flex32 » (+24 −2) : nos PR 2 et 4 sont de cette
   famille. cubek#639, « accumulate adaptive_avg_pool2d […] in f32 for narrow
   floats » : son `accumulator_dtype` promeut déjà f16, bf16 **et flex32** ;
   notre PR 3 aligne le matmul sur cette règle.
4. **Les PR 2, 4 et 7 se reproduisent sans Vulkan** : sur wgpu en WGSL, Flex32
   est déjà inscrit. Elles peuvent donc être rouges puis vertes sur leur
   `main` tel qu'il est, ce qui est la preuve qu'ils attendent.
5. **Voisins à connaître.** burn#5953 (brouillon, attention à requêtes
   groupées) ne touche pas `kernel/attention/base.rs`. cubek#519 (f16 faux sur
   une convolution 1×1, Vulkan RADV) est sans réponse depuis le 20 août : ce
   n'est pas notre défaut, mais c'est le même matériel.

## Réserves

- Le champ `author_association` de GitHub range parmi les « contributeurs »
  trois personnes qui agissent en mainteneurs (Charles23R, SamuelBelanger,
  marcantoinem) : les chiffres « extérieurs » de cubek en sont gonflés.
- Les délais sont calculés sur les envois lus, pas sur les 300.
- Non lus : le `GUIDE.md` de cubek, le livre du contributeur de burn, leur
  Discord (où une partie de la coordination a lieu).
