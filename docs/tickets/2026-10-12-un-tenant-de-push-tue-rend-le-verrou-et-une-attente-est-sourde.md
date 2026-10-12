# Un tenant de push tué rend le verrou pendant que sa commande pousse, et une attente de poste ne s'annule pas

- **État** : ouvert (12 octobre 2026, optimiseur, à la demande de l'orchestration)
- **Gravité** : réponse fausse (deux pushes en même temps, `--qui` qui ne nomme que le second) ;
  blocage (une attente qui ne s'annule pas, et un `flock` mort qui laissait la commande partir)
- **Atteignable en service** : oui, sur les deux postes
- **Touche rag3weaver** : oui (`extension/rag3weaver/scripts/poste`)

## Ce que c'est

Trois défauts du même script :

1. **Le verrou de push était tenu par un `flock` qui enveloppait le tenant.** Un `flock` tué
   (le cœur a arrêté une file « par ses pids ») rendait le verrou pendant que la commande,
   orpheline, poussait encore. Deux pushes tournaient alors en même temps, et `poste push --qui`
   ne nommait que le second.
2. **Une attente de verrou était sourde.** `poste` attendait dans un `flock` au premier plan, et
   un signal n'était traité qu'au retour de `flock`. Deux bâtis annulés sur luciepc sont restés
   bloqués jusqu'à ce qu'on tue leur `flock` à la main.
3. **Un `flock` qui échouait ou mourait laissait le script continuer** jusqu'à la commande : un
   lourd entrait sans porte, une mesure sans exclusif.

**Incident du 12 octobre, 3 h**, en écrivant le témoin de ce ticket. Ses cas 5 et 6 terminaient
par motif sur tout le poste (`flock … master-push.lock`, puis `flock -x 8`). Ils ont tué trois
pushes de codeparsers, un de F et un de la recherche : tous morts avant `git push`, rien de
perdu, et les sessions ont vérifié. Le témoin ne vise désormais que ses propres pids, sur ses
propres verrous.

## La recette

- `poste push A -- <commande longue>`, puis TERM au `flock` qui tient le verrou ; un second push
  passe pendant que la commande de A tourne encore.
- `poste lourd …` derrière une mesure, puis TERM au script : il ne sort qu'à la fin de la mesure.

## Le témoin

`extension/rag3weaver/scripts/test_poste.sh`, cas 5 et 6.

## Ce qu'il faut pour le fermer

- Le verrou de push tenu par le script lui-même (descripteur 7), et un signal qui ne le rend
  pas avant la fin de la commande.
- L'attente par `flock` en fond et `wait`, qu'un signal interrompt ; un `flock` en échec arrête
  le script.
- `--qui` qui nomme le tenant, ou, sans fiche, le pid du verrou et sa commande.
