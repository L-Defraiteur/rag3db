# Une seconde carte d'accord arrive parfois trop tard dans une fenêtre qui n'est pas le chat

- **État** : ouvert, le 23 novembre 2026
- **Famille** : produit
- **Gravité** : basse : un témoin navigateur qui rougit parfois ; rien n'est montré faux à l'utilisatrice, une carte arrive après le délai du témoin
- **Atteignable en service** : à voir (le témoin attend 20 s ; une carte qui arrive plus tard s'afficherait quand même)
- **Touche rag3weaver** : oui (`src/daemon_host/chat_contextuel.rs`, `src/daemon_host/navigateur.rs`)
- **Pour** : recherche (l'IDE)
- **Signalé par** : archi, passe du lot `lands-what-commands-made` à luciepc, le 23 novembre 2026

## Ce qui s'est passé

`daemon_host::chat_contextuel::une_fenetre_qui_n_est_pas_le_chat_compte_les_accords_en_attente` a rougi une fois,
à `src/daemon_host/navigateur.rs:441` :

> a second card awaits its approval : « document.querySelectorAll('details.outil[data-outil=run_command] +
> .ui-du-tour .carte-accord').length === 2 && document.querySelector('#status')?.textContent === 'fini' » jamais vraie
> en 20 s (20 s de calme)

Passe sur la tête a1c8e72f4d (lot `lands-what-commands-made`, qui ne touche ni les cartes ni la vue) : 72 témoins
navigateur verts, celui-là rouge ; preuves `~/.cache/rag3weaver-build/validations/validation-u3lkatdp` à luciepc.

## Les rejeux

La série entière des 73 témoins navigateur (`scripts/temoins_navigateur.sh`), à luciepc, juste après :

| tête | essai 1 | essai 2 |
|---|---|---|
| master a36651f88c | 73/73 | 73/73 |
| a1c8e72f4d | 73/73 | 73/73 |

Aucun rouge reproduit en quatre passages.

## À faire

Lire, dans la passe rouge, si la seconde carte manquait ou si `#status` n'était pas encore « fini » ; et si l'attente
du témoin porte sur l'événement (la carte posée) ou sur une durée de calme qu'une machine chargée peut dépasser.
