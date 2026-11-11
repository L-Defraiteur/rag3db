# Une invocation peut exécuter une autre version que celle vérifiée

- **État** : ouvert (11 novembre 2026), point de revue sur le lot contextuel en branche ; entrelacement à éprouver avant intégration.
- **Famille** : produit
- **Gravité** : la vérification de lecture seule pourrait ne pas couvrir le graphe effectivement exécuté après un rechargement concurrent.
- **Atteignable en service** : potentiellement avec le futur relais et un rechargement concurrent ; non reproduit à ce stade, relais non livré sur master.
- **Touche rag3weaver** : oui, Backend::invoke_read, call_in et version des déclarations.
- **Pour** : déclaratif, dans la revue du lot d’invocation contextuelle.
- **Signalé par** : orchestration ; lecture de la source commune 458e50b46a277618f08bb0f37d5bcbebe1250db7 après ses validations sur a92.

## Faits lus et hypothèse à départager

Dans cette source, `Backend::invoke_read` prend `let prepared = self.prepared()`, puis vérifie
sur cette version l’exposition de l’opération, sa politique, son harnais, ses bindings, le
graphe instancié, `snapshot_safe` et le crochet after. Il appelle ensuite `self.call_in(...)`.

`call_in` pose la cellule puis passe par `call` ; `call_tool` rappelle `self.prepared()` avant
`call_tool_on`. Le pointeur préparé qui a subi les vérifications n’est pas transmis à cette
exécution. `prepared()` clone l’Arc sous un verrou relâché immédiatement.

Le backend sait remplacer ses déclarations par `apply_reload`. Les chemins `declare_set` et
`apply_reload_with_reactions` peuvent le faire sur un `&Backend`. Le verrou de lecture du slot
dans `HostedInvocations::invoke` maintient l’instance ouverte ; il ne prouve pas à lui seul
que cet Arc de déclarations reste identique entre les deux lectures.

L’entrelacement envisagé est : vérifier une opération en lecture seule sur A ; recharger sa
déclaration sur B ; exécuter B par `call_in`. Si B porte d’autres effets, les vérifications de
A ne suffisent plus. La fenêtre est établie par la lecture des fonctions ; son atteignabilité
exacte dans le parcours hôte et le résultat d’un témoin déterministe restent à vérifier.

## Travail attendu

Départager ce point en préservant la source jugée et les preuves précédentes. Le témoin doit
contrôler le moment du remplacement, plutôt que compter sur des délais ou une course aléatoire.
Si le chemin permet cet entrelacement, l’invocation doit vérifier et exécuter une même version
des déclarations, ou refuser explicitement le changement, tout en conservant cellule, gardes,
annulation et présentation ordinaires. Ne pas contourner leurs contrôles en appelant une voie
d’exécution moins protégée.

## Pour fermer

Soit une preuve précise établit pourquoi le remplacement ne peut pas intervenir sur les chemins
concernés, soit un témoin rouge puis vert prouve qu’une opération différente, non admissible
en lecture, n’est jamais exécutée après la validation de A. Citer la source, les résultats et
les contrôles rejoués. Ce suivi complète le
[ticket des outils contextuels](2026-11-11-le-chat-de-l-atelier-n-expose-pas-les-outils-de-recherche-ni-wait-output.md)
et reste distinct des problèmes d’accord de commande hors domaine.

## Départage et correction en branche

ED reprend ce suivi. Le témoin déterministe `949dcb556` capture A (`SchemaNode`),
remplace réellement les déclarations par B (`RhaiNode`) avec `apply_reload`,
prouve que l'invocation directe de B est refusée, puis observe que l'appel contrôlé
sur A exécute néanmoins B (`result.version = 2`). Rouge : 0 réussi / 1 échec,
0,01 s, preuve externe `I-luciepc/relais-20261111/version-rouge-949dcb556.log`.
Deux essais de montage antérieurs échouaient sur les paramètres du témoin et ne
constituent pas cette preuve.

`60bffb237` fait circuler la même référence préparée dans `call_in_on`, le montage
des réactions de cellule, `call_on`, la garde agent et `call_tool_on` ; cellule
RAII, annulation parent, harnais et présentation restent sur le parcours commun.
`074b3dced` compare le résultat au rendu réel de A : ciblé vert 1/0/0, 0,01 s.
Une assertion intermédiaire attendait à tort un objet de SchemaNode, qui rend du
texte ; son échec reste distinct du rouge de sécurité. Preuve verte externe :
`I-luciepc/relais-20261111/version-vert.log`.

Reste : passe commune complète (dont rechargements et harnais ordinaires), trois
parcours UI réels et intégration autorisée. Ticket encore ouvert jusque-là.
