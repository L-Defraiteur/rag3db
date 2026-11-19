# La campagne d'Algareth, rejouée le jeudi 19 novembre 2026

Ce sont les demandes de c7 du 16 novembre (`docs/orchestration/16-11-2026/01-essais-empiriques-algareth.md`),
reprises mot pour mot.
- **Où.** L'hôte de la tablette (7970), posé ce matin, dans la tâche `archi-essai` sur le dépôt de
  Lucie (branche `agent/archi-essai` depuis `cd133788a`). Algareth répond par gemini-3.8-flash.
- **Comment.** Avec `rag3weaver-tty --plain`. Les appels se relèvent dans le journal des runs
  (`runs/a7b6ccfed4ef`).
- **Ce qui est mesuré : pas-va-au-bout et S1**, qui sont sur ces hôtes. S2, S3 et le rangement au
  fil sont sur master depuis 11 h 17, mais les hôtes ne les portent pas encore. Ils arrivent à la
  pose suivante, et D5 et D6 se rejouent alors.

## Le relevé

| n° | réponse | voie | pas | durée | par rapport au 16 |
|---|---|---|---|---|---|
| D1 | `backend.rs:1146`, **juste** | grep (1 appel suffit), puis 7 appels de trop : la page de la campagne, teammate.rs, un grep refusé à 1 126 lignes, search_code | 1 | 122 s, 150 k jetons d'entrée | **mieux** : `report` avant `step_done`, pas de présentation, 1 pas. Reste **S6** : il lit ce qui parle de lui |
| D2 | les 3 appelants, **justes** | `usages`, puis un grep et une lecture de confirmation | 1 | 35 s | **mieux** (47 s) ; reste **S5** : « Je suis Algareth. » |
| D3 | les 13 `pub fn`, **justes** | grep, `outline` ×2, une lecture | 2 | 42 s | même voie ; reste **S4** : `step_done` avant `report`, relancé |
| D4 | les 9 lignes, **identiques** | `read_file` par offset, refait au pas 2 | 2 | 25 s | **S4** encore (relance), **S2** (le pas 2 relit) |
| D5 | `backend.rs:1163`, `NOM_DE_L_AGENT_PAR_DEFAUT`, « équipier », **juste** | 5 greps, 2 lectures, **aucun `search_code`** sur une question de sens | 2 | 109 s | **S4**, **S5** ; le pas 2 refait les greps (**S2**, pas encore sur l'hôte) |
| D6 | **non jouée** : voir plus bas | | | | |

## Ce qui va au bout, ce qui s'arrête, les bornes

- **Va au bout.** Les cinq demandes de lecture finissent seules, par « fait sans plan », sans
  attendre le superviseur. Pas-va-au-bout tient : aucun pas ne s'arrête sur « ce qui reste au
  prochain pas ».
- **Les bornes.** Aucune n'est atteinte : 64 k de contexte, 60 appels d'outils, 40 appels au
  modèle. Le plus long, D1, fait 11 appels au modèle en un pas, au-delà de l'ancien plafond de 12
  d'outils et de modèle qui le coupait le 16. Un grep au-delà de 1 000 lignes est refusé par sa
  garde, qui suggère `search_code`. En D1, Algareth l'a pris ensuite.
- **S'arrête.** Rien ne s'arrête, mais deux défauts restent en tête de liste :
  - **S4**, `step_done` avant le `report` : 3 demandes sur 5, chacune avec une relance de 11 à 33 s ;
  - **S5**, la présentation : D2 et D5.

## D6

- **Le 16**, à la tablette, D6 s'est perdue dans les lectures (S1).
- **Aujourd'hui, D6 n'est pas jouable à la tablette** sans risque pour l'écran de Lucie. La porte
  laisse passer `cargo test` hors de `poste lourd` : un bâti de rag3weaver à pleine charge, sans
  deux cœurs libres. c7 en a fait un lot pour archi : les commandes lourdes de la porte sous
  `poste lourd`.
- **À luciepc**, D6 a été lancée sur l'hôte de luciepc, par un tunnel. Le dépôt « rag3db » de cet
  hôte est l'arbre de Lucie là-bas : figé à master du 18 octobre (`284cc00c0`), sans la fonction,
  et déclaré `read_only` avec `commands: "off"`. Algareth l'a constaté juste, en un pas, puis a
  fini. Ce n'est pas une mesure. Personne ne touche à cet arbre.
- **D6 se joue donc à la tablette**, après le lot de la porte et la pose qui portera S2 et S3.

## La suite

1. Le lot de la porte : les commandes lourdes sous `poste lourd`, rouge d'abord.
2. S4, `step_done` avant le `report`, et S5, la présentation : les entrées suivantes de la liste.
3. D1 à D6 rejouées à la pose suivante.

Les journaux et les traces sont dans le bac d'archi (`algareth/d1…d6.*`).
