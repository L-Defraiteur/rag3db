# Clé du débit par fournisseur appelé et par poste

Ticket : [débit par service](../tickets/2026-11-16-le-debit-d-embarquement-par-service-pour-le-seuil-de-6-s.md).
Lot borné autorisé par orchestration : **clé et lecture seulement**, reprise des
3 témoins rouges `0aecc6b70b116c08ff8f266bf86370090b32fc77`.
Base relue : `927941a67a5805450d49f1e04b39fdd542a6059b`.
Branche de reprise : `cx-rate-key-20261116-r1`, W local
`/home/lucied/git_workspaces/cx-rate-key`. L'ancien W rate-key reste intact.

## Contrat

- Clé des mesures automatiques : `embedding_rate:{model}:{address}@{host}`.
  Un embarqueur réellement en processus utilise `local@{host}`. Le poste est
  celui du client ; un tunnel constitue volontairement une provenance distincte.
- `Embedder::called_address()` est une **lecture sans effet** : l'adresse du
  client simple, celle de la prise actuelle de `DenseTenu`, sinon `None`.
  Elle ne prend aucun bail, ne démarre aucun fournisseur et ne choisit pas un repli.
- `None` n'est local **que si `distant()` est faux**. Distant sans adresse prise
  (avant premier appel ou après restitution du bail) reste inconnu : lecture de
  l'ancienne clé `service` en secours, jamais de clé `local@host` inventée.
  Une mesure sans provenance connue est refusée explicitement, sans écriture.
- `this_host()` lit `gethostname`, sinon `/etc/hostname`. Si les deux échouent,
  l'hôte reste inconnu : pas de sentinelle persistée. Même règle d'absence de
  nouvelle clé ; la lecture historique reste possible.
- `known_embedding_rate` préfère la nouvelle clé courante, puis l'historique
  `{model}:service|local` si la nouvelle manque. Le modèle non configuré ou
  configuré indisponible ne gagne aucun débit, comme avant.
- `note_measured_embedding_rate` et `probe_embedding_rate` écrivent uniquement
  la nouvelle clé, la provenance étant lue après le calcul de la sonde.
- `embedding_rate_at` et `note_embedding_rate_at` offrent au banc l'accès exact
  à une provenance déjà qualifiée `address@host` ; aucune lecture d'une autre
  adresse ou d'un autre poste ne se mélange.

Les méthodes publiques historiques `embedding_rate` et `note_embedding_rate`
restent inchangées pour les appelants explicites existants (dont les témoins
qui préparent une ancienne clé). « Ne plus écrire l'ancienne clé » porte sur
les mesures automatiques et le banc via les méthodes `_at`, pas sur la
suppression d'une API existante. Aucune migration/destruction des métadonnées.
Le banc attend cette lecture sur master avant d'écrire au catalogue.

## Invariants et limites

Sans modèle, aucun nom, dimension ou débit fabriqué. Les wrappers `Arc` et
`DenseTenu` relaient les identités sans changer l'exécution d'embarquement.
L'absence d'adresse ne déclenche pas d'activation pour rendre une estimation.
`Rate` et sa sérialisation numérique restent inchangés, ainsi que la ligne
française d'avancement parsée par l'hôte. Aucun changement de `index_nodes.rs`,
aucun étalonnage ni mesure par passe, aucune table textes/s/date dans ce lot.
Le seuil de six secondes et la politique des vues restent aux porteurs dédiés.

## Preuve et qualification à venir

Les trois rouges hérités sont préservés avec leur commit et le journal PC
`~/.cache/rag3weaver-build/rate-key-red.out` ; aucun nouveau rouge n'est prétendu
joué pendant la fenêtre de bascule. Compléments hermétiques : provenance
inconnue distante, priorité/isolement adresse et poste, relai Arc, absence
lexicale, lecture de l'adresse du client et activation sans appel au fournisseur.

Aucun Cargo, test ou service réel tant que la fenêtre G/Arbre n'est pas rendue.
Le WIP non compilé est poussé avant la recette. Après « FINI G », proposer une
fenêtre neuve, vraie portée commune/poste lourd/Max32G/j8/target unique/lib
épinglée du moment ; ciblés, puis F5/preuve/lib entière/NAV et suites
`e2e_code_sync`, `e2e_lectures`, `e2e_embarqueur_a_la_demande`. Les usages de
services de cette future passe restent coordonnés ; aucun ancien GO/délai.
Le push master attend un nouveau « à toi » après qualification.
