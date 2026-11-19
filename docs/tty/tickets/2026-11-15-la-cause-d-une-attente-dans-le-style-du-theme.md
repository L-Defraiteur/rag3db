# La cause d'une attente, dans le style du thème et telle que l'hôte la dit

- **État** : ouvert. Le style du thème est fait (tty-25, 18 nov. 2026). Les attentes que seul l'hôte connaît sont faites le 19 nov. 2026 : l'événement par `cx-hote-waiting` (page `docs/tty/19-11-2026/05-l-hote-dit-qu-un-tour-attend.md`), le terminal par tty-29 (page `docs/tty/19-11-2026/06-tty-29-l-attente-dite-par-l-hote.md`). Il se ferme au « poussé » des deux.
- **Famille** : produit
- **Gravité** : gêne (Lucie ne voit pas ce que l'équipier attend quand c'est un quota ou l'index).
- **Atteignable en service** : oui, à chaque tour de l'équipier.
- **Touche rag3weaver** : oui (le crate `extension/rag3weaver/tty`, et l'hôte pour l'événement).

## Ce que c'est

Lucie, 15 nov. 2026 : « je veux voir l'animation et comprendre ce qu'il attend exactement (si c'est quota ou quoi que ce soit, je veux l'indication à côté de l'animation), voire même que ça fasse partie de l'animation : en gros à côté, mais là aussi des glitchs ou selon le thème ».

Fait : à côté de l'animation, en sourdine,
- « le modèle répond… (gemini-3.8-flash, 12 s) », par le dernier `LlmCall` et le temps depuis le départ de l'appel ;
- « run_command cargo check travaille… (40 s) », par l'appel en cours et sa cible ;
- « attend ton accord : … [ Oui ] Non », déjà fait.

Reste :
1. **La cause rendue dans le style du thème.** Aujourd'hui elle est en sourdine, d'un seul style. Il faut qu'elle prenne les glitchs en alien et la forme propre à chaque thème. Il faut aussi choisir dans les thèmes comment chacun la dit, et garder le coût en octets sous le plafond (`tui::tests::mesure_des_octets_*`).
2. **Les attentes que seul l'hôte connaît** : un quota du fournisseur avec son délai de réessai, l'index qui termine, une commande en fond. c7 a demandé à archi et à ED un événement de la boîte `waiting {cause, since, retry_in?}`, effacé quand le travail repart. Le terminal le lira quand sa forme sera posée.

## Ce qu'il faut pour le fermer

L'événement de l'hôte posé, puis lu par la vue. Chaque thème rend la cause dans son style, et le harnais montre la cause d'un quota simulé par le faux hôte.
