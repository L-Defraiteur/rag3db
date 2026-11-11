# Invocation contextuelle d'une opération déclarée — contrat à éprouver

Préparation indépendante des lots réclamations et remaining explicite. Ce document ne décrit
pas une capacité livrée. Archi tient le parcours navigateur ; ED les contrats et primitives.
Les fichiers code.rs et le gabarit code restent réservés au chantier des vues.

## Primitive et composition

Une invocation porte une cible déclarée backend/espace, une vue éventuelle, une opération fixée
par la déclaration et des arguments typés. Le dépôt est une sorte de contexte, pas une obligation
pour tous les domaines. Le modèle choisit des arguments dans les limites du contrat exposé ; il
ne fournit pas un nom d'opération arbitraire permettant de contourner ce contrat.

Le graphe de l'atelier peut déclarer les outils search_code, outline et les autres capacités
par cette primitive, sans dispatcher spécial selon leur nom. L'opération reste celle de la cible,
avec ses validations, sa recherche et ses rendus ; aucun second moteur réservé à l'atelier.
Le schéma exposé doit valider contexte et arguments cibles ; pas un sac JSON libre sans contrôle.
Un contexte inconnu, ambigu ou incompatible avec une vue refuse sans repli vers le premier dépôt.
Une vue non implémentée doit être refusée, jamais ignorée.

wait_output se compose séparément avec tools/wait.mmd et WaitOutputNode existants, en gardant
la restriction aux journaux que le runtime autorise. Ajouter ce graphe n'exige pas à lui seul
une primitive de délégation ni un accès arbitraire aux fichiers.

## Autorité, contexte et durée de l'appel

Le relais n'élève jamais les pouvoirs : autorité de l'appelant ET autorité de la cible doivent
permettre l'opération. Personne, domaine, périmètre, tâche et contexte agent/humain restent
conservés. Les politiques de lecture seule et task_writes continuent de s'appliquer à la cible.
Ne pas transformer une invocation d'agent en call_humain ou call_discret ; ne pas copier un accord
d'un contexte à l'autre par simple changement de cible.

La validation doit vérifier les capacités réelles de l'opération cible, pas seulement l'étiquette
lecture du wrapper : une déclaration qui cible une écriture en se disant lecture doit refuser.
Les mêmes accords/refus, erreurs et mécanismes d'annulation restent observables. Une annulation
du parent atteint l'enfant et n'autorise pas une réponse de succès de substitution.

Le reçu conserve résultat, présentation, métadonnées et UI utiles. Sa provenance relie run et
appel parents à l'opération, au backend/espace et à la vue effectivement résolus. Ce contexte
est structuré par le runtime, avec un libellé humain pour l'écran : le renderer ne doit pas
résoudre les liens, cartes et actions depuis l'identifiant de la conversation parente.
Deux dépôts peuvent partager chemin et symbole sans partager cette cible. Le contexte effectif
survit à la conservation de l'historique, au rechargement et à la reprise ; une instruction
textuelle du modèle n'est pas une autorité pour le reconstruire. Les refus
portent eux aussi ce contexte sans afficher des valeurs de secrets.

L'auto-appel et les cycles ne sont pas permis implicitement. Résoudre et vérifier les références
avant de prendre des verrous imbriqués ; un cycle refusé ne doit pas devenir un blocage du catalogue.
Une éventuelle récursion autorisée nécessiterait un contrat de budget explicite, hors premier lot.

## Points existants à réutiliser et écart à mesurer

WorktreeHost fournit le service de résolution et Backend::call le chemin d'appel d'agent.
Les lecteurs atelier actuels composent déjà des appels read_file/usages/outline spécifiques.
NodeContext porte run_id et mécanismes d'annulation ; cela ne prouve pas qu'un contexte complet
d'autorité se transmet aujourd'hui à un appel enfant. Vérifier cette transmission avant de livrer
le relais. Ne pas déduire l'autorisation d'un accès au seul service hôte.

L'inventaire effectif navigateur d'archi distingue atelier, dépôt hors tâche et tâche. Les absences
actuelles de l'atelier sont un état à corriger ; les restrictions d'écriture/commande du dépôt
hors tâche sont une politique à préserver.

## Preuves avant intégration

1. Rouge d'exposition du modèle puis vert lorsque les outils déclarés deviennent accessibles.
2. Exécution réelle de recherche et outline sur deux dépôts de contenus distincts : cible demandée
   respectée, résultat et provenance corrects, zéro fuite vers l'autre dépôt. Avec mêmes chemins
   et symboles mais contenus distincts, cliquer un lien doit ouvrir la cible demandée, encore
   après rechargement/reprise. Archi tient Chat.tsx/toolview.tsx et le témoin chat_contextuel.rs ;
   le branchement du module sera coordonné, aucun de ces fichiers n'est édité par ED.
3. Référence inconnue/ambiguë, vue non supportée, auto-appel/cycle : refus observable, sans blocage.
4. Appelant lecture seule ou cible lecture seule face à une écriture : refus dans les deux sens ;
   tâche autorisée conservant ses capacités, aucune conversion en appel humain.
5. Erreur cible et annulation parent : propagation et arrêt réellement observés, sans succès inventé.
6. Commande différée : wait_output observe sa sortie finale ou son erreur par le graphe existant,
   et refuse un journal hors périmètre. Le navigateur garde actions et état après reprise.

Archi et ED coordonnent les fichiers des témoins de parcours. Le ticket atelier reste ouvert tant
que l'exécution depuis le chat, les gardes et la reprise ne sont pas démontrées ; un inventaire
seul ne ferme pas ce ticket.

## Frontière JSON minimale convenue avec archi

Premier lot : cible `backend` seulement. Une cible `space` ou une vue non encore disponible
refuse explicitement. Le contexte sélectionné ne remplace pas celui de la conversation parente.
Les noms/types/défauts et bindings des paramètres de l'opération cible sont conservés ; pour
search_code, `target=Scope` reste le binding du manifeste, distinct de `context.target`.

Appel search_code, la requête s'appelle `query` comme dans le graphe livré :

```json
{"context":{"target":{"kind":"backend","id":"B"}},"query":"symbole_commun","options":{}}
```

Appel outline :

```json
{"context":{"target":{"kind":"backend","id":"B"}},"path":"src/lib.rs"}
```

Le reçu porte le contexte effectivement résolu dans sa métadonnée structurée :

```json
{"metadata":{"invocation":{"target":{"kind":"backend","id":"B","label":"dépôt B"},"parent_run":"run-parent","run":"run-cible"}}}
```

`view`, lorsqu'elle sera supportée, est optionnelle dans `context` et dans `metadata.invocation`,
avec le contrat de vue partagé ; elle est absente du premier cas nominal. Le label vient du
runtime, sans nom technique de backend à afficher. Le renderer choisit cette cible pour les
liens et actions du résultat, y compris après rechargement. Le témoin navigateur clique uniquement
sur le lien produit par le résultat réel, sans href injecté ni route de test de substitution.
