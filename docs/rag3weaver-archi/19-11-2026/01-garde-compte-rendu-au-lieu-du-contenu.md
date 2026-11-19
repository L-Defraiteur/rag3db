# La garde « un compte rendu n'est pas le contenu demandé » : la conception d'archi — le jeudi 19 novembre 2026

Ticket : `docs/rag3weaver-archi/tickets/2026-11-14-l-equipier-rend-un-compte-rendu-au-lieu-du-contenu-demande.md`.
La proposition et le témoin sont de tty : `docs/tty/19-11-2026/04-proposition-de-garde-compte-rendu-au-lieu-du-contenu.md`
et `a_report_about_the_text_is_not_the_text`, sur la branche `cx-equipier-compte-rendu` (`31603f67a`).
Je prends ses trois couches, et je tranche ses trois points.

## Les trois points

1. **Qui juge : le modèle de l'équipier lui-même**, pas le juge des bilans. Le juge de `juge.rs`
   travaille en fond, après le run, sous un plafond horaire (`bilans.max_per_hour`) : il ne sait
   pas répondre à une frontière, sur le moment. Le décideur local n'existe pas encore partout (graine
   Laya). L'appel :
   - un seul appel au modèle de la tâche, à température 0 et 50 jetons de sortie au plus ;
   - une question fermée en anglais (texte d'hôte vers le modèle) : la request, les reports et le
     texte du pas, puis « Does the request ask for a text, and is that text in the step's report or
     answer? Answer with one word: delivered, missing or action. » ;
   - il ne se fait que dans le cas étroit de la couche 1.

   Un appel en échec (quota, panne) ne bloque rien : l'hôte écrit « fin sans texte, à vérifier » et
   la tâche finit comme aujourd'hui.
2. **La relance compte dans `MAX_STEPS`**, c'est un pas. Elle **partage le budget de relance
   unique de la garde B** : une request ne se relance qu'une fois en tout, quelle que soit la garde
   qui relance. Au second manque, l'équipier attend le superviseur, borné par
   `teammate.reply_wait_s`, comme sous `expects: "report"`.
3. **Le terminal et l'IDE ne posent pas `expects` d'eux-mêmes.** D'accord avec tty : le
   superviseur ne le sait pas mieux que le juge.

## Où dans le code

- `teammate.rs`, à la fin d'un pas déclaré réussi.
- La couche 1 se lit sur ce que le pas a déjà noté : pas de texte de modèle, des reports de moins
  de 300 caractères, et une request sans `expects`.
- L'appel passe par le même `ChatModel` que le pas, hors du tour : il n'entre pas dans la session
  de la tâche, et il est journalisé comme un appel de l'hôte.

## La preuve

- Le témoin de tty, tel quel.
- Un second témoin : un faux juge qui répond `action` laisse finir la tâche (le cas sain ne
  change pas).
- Un troisième : un juge en panne donne « fin sans texte, à vérifier ».
- Puis la passe, avec e2e_equipier sous Vertex : le témoin réel `l_equipier_rend_le_texte_demande`.

## Taille

1 jour. Pour tty, s'il garde le lot ; je relis le diff.
