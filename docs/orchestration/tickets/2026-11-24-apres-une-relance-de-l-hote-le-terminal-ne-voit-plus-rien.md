# Après une relance de l'hôte, le terminal ne voit plus rien et ne se quitte plus

Ouvert le 24 novembre 2026, 13 h 50. Vu par Lucie dans `tisserande`, terminal ouvert pendant la relance de l'hôte
(13 h 45 min 51).

## Le constat

- Le terminal dit « flux coupé (l'hôte a fermé le flux) : je reprends ». Lucie pose une question à 13 h 46 min 06 ;
  l'agent répond à 13 h 46 min 32 (la réponse est dans la boîte, juste et complète). **Le terminal n'affiche rien** et
  reste sur « YOUR MOVE ».
- Ctrl-C, plusieurs fois : « [tty] arrêt demandé » à chaque fois, et **le terminal ne se quitte pas**. Il croit le
  tour encore en cours (il n'a pas vu sa fin), donc Ctrl-C arrête le tour au lieu de quitter.

## La cause probable (à prouver)

Le terminal reprend le flux par `Last-Event-ID`. Un hôte neuf recommence sa numérotation : les événements du tour
portent des numéros plus petits que le dernier vu, et ne sont jamais montrés.

## Ce qui est attendu

1. Le flux dit l'identité de l'hôte (son heure de départ ou un identifiant d'époque) ; un terminal qui reprend sur un
   hôte neuf repart de zéro, relit la boîte de sa conversation depuis le disque (comme `--continue`) et montre ce qu'il
   a manqué.
2. **Quitter marche toujours** : Ctrl-D, ou `/quit`, quitte quel que soit l'état ; et trois Ctrl-C de suite quittent
   même pendant un tour (le premier arrête le tour, comme aujourd'hui).

## Le témoin

Par un vrai hôte : un terminal ouvert, l'hôte relancé, une demande ; la réponse s'affiche. Puis Ctrl-D quitte pendant
un tour.
