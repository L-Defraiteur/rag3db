# Un accord de commande hors domaine redemande le même accord

- **État** : ouvert (11 novembre 2026), relevé par archi, transmis à ED.
- **Pour** : ED (décision et reprise de commande), prise en charge confirmée ; transfert de dossier après la passe figée.
- **Signalé par** : archi, navigateur `b57313175` et journal ci-dessous.
- **Famille** : produit
- **Gravité** : l'accord annoncé dans la page ne permet pas de reprendre la commande.
- **Atteignable en service** : oui, carte d'accord de commande du chat atelier.
- **Touche rag3weaver** : oui, décision de commande et reprise après accord.

## Preuve

Sur `b57313175`, le témoin navigateur de commande différée place son script Python dans le
parent de la racine de l'atelier. Le premier appel `run_command` demande l'accord. Le clic réel
sur « accorder » affiche « accordé » et relance le tour avec exactement la même ligne. Le second
appel redemande l'accord ; aucun `wait_output` n'est atteint. Aucun contenu privé n'est impliqué.

Journal luciepc :
`~/.cache/rag3weaver-build/cx-ui-contextuel-b57313175-coupure-attente-20261111/navigateur.log`.
Témoin : `le_chat_attend_la_sortie_differee_d_une_commande_apres_accord` avant déplacement de
son script sous la racine. Rouge conservé ; la fixture suivante sépare ce défaut de l'attente.

La lecture de `commande.rs` montre que `accorder_ligne` enregistre la commande dans les acquis,
mais `juger` contrôle les arguments hors domaine avant ces acquis. Ce mécanisme explique le
comportement observé ; ne pas supprimer la frontière du domaine pour corriger la reprise.

## Pour fermer

Définir et prouver la portée de l'accord explicite sur cette ligne et ce domaine, puis une
reprise qui l'honore sans accorder d'autres chemins ni variantes de commande. Le refus doit
rester observable si cette autorisation n'est pas prise en charge. Tester la carte et le tour
réel, pas seulement l'enregistrement de la décision. Ce correctif est distinct du lot UI
contextuelle et ne doit pas être annoncé livré par le succès de wait_output dans le domaine.
