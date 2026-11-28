# `--plain` après une relance de l'hôte perd les lignes de l'intervalle

- **État** : ouvert le 28 novembre 2026 par la session tty, après `tty-new-host-epoch` (la vue interactive reprend sur un hôte neuf et relit sa boîte ; `--plain` ne le fait pas encore).
- **Famille** : produit
- **Gravité** : défaut — en `--plain`, après une relance de l'hôte, la réponse donnée pendant la relance peut ne jamais s'écrire, et la séance peut ne pas voir la fin de son tour.
- **Atteignable en service** : oui (un `--plain` ouvert pendant une relance de l'hôte).
- **Touche rag3weaver** : oui (le crate `extension/rag3weaver/tty` : `main.rs`, la boucle de `--plain`).

## Ce que c'est

`flux::suivre_sans_fin` reconnaît depuis le 28 novembre un hôte neuf à son époque : il reprend sans `Last-Event-ID` et
passe au lecteur une trame `host_restarted`. La vue interactive relit alors la boîte de sa conversation (elle ne
réécrit que les lignes qu'elle n'a pas vues, par leur `id`).

`--plain` ne réagit pas à `host_restarted`. Sa relève de la boîte (toutes les 2 s) n'écrit plus rien dès que le flux
relaie `MailboxLine` (`boite_au_flux`), mais elle compte quand même les lignes lues (`lues`). Les lignes écrites
pendant la relance sont donc comptées sans être écrites, et le flux de l'hôte neuf ne les rejoue pas.

## Ce qui est attendu

Sur `host_restarted`, `--plain` relit la boîte de sa tâche et écrit les lignes qu'il n'a pas encore écrites (par leur
`id`, comme la vue), puis suit comme avant ; la fin de son tour (`ended`) est vue, et la séance rend la main.

## Le témoin

Par le script de mesure du crate (`mesures/faux_hote.py` sait servir une époque) : un `--plain` sur une demande, le
faux hôte relancé avec une autre époque au milieu du tour, une ligne de boîte écrite pendant la relance ; elle
s'écrit une fois, et la séance finit.
