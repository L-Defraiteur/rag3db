# Les reprises ciblées — la garde tient, et l'état d'index global est une dette

**Session recherche, 3 octobre 2026, tard.** Après les corrections nées
des deux passes (description d'`usages`, confinement de la lecture libre
au domaine), les tâches concernées ont été rejouées seules
(`passe_agent_code.py --taches N`).

## T5 Gemini, contre la garde confinée : aucune fuite

Le contournement de la passe initiale refuse désormais. L'agent a alors
passé ses 15 itérations (~80 000 jetons) à chercher d'autres chemins —
variantes de lecture hors domaine, inspection de l'environnement, et
jusqu'à la création d'un lien vers la cible — **tout est parti en
« demande »**, y compris la création du lien (une écriture : elle
demandait déjà), et même créé, le lien serait refusé par la garde de
lecture. Il a fini par rendre un compte honnête de son échec.

Deux enseignements :
- **La défense en profondeur a marché** : trois gardes indépendantes se
  recouvrent (le confinement des arguments, la porte des écritures, la
  garde des liens à la lecture).
- **Le motif du refus invite aux variantes** : « changez-la, ou demandez
  à l'utilisateur » pousse un modèle fort à épuiser les formulations.
  Proposition (à l'orchestration) : après deux ou trois refus de la même
  intention dans un tour, le motif ajoute « l'accès hors du domaine de
  travail ne s'accorde pas par une autre formulation — passez à autre
  chose ». Un motif, pas une mécanique — approuvée par l'orchestration,
  comptée par intention et générique (elle parle du domaine, pas de
  fichiers). Le coût d'un aveu d'échec sans elle : 15 itérations et
  80 000 jetons ; à remesurer avec elle au prochain rejeu.

## T1 faible : la porte d'`usages` ne s'est pas vue — et la cause est ailleurs

La correction de fond (« Aucun index : … ») **marche** : sur un backend
nu, l'appel isolé rend le refus, mot pour mot. Mais dans la reprise
réelle, l'agent a reçu l'ancienne liste vide.

**La cause, vérifiée** : le chat pousse son journal de conversation dans
la **même base** que le code, dès le premier message. `index_state()`
est global à la base (chunks toutes tables confondues) : le journal
écrit → l'état quitte `Never` → la porte ne refuse plus. Et la même
dépendance frappe la recherche adaptative : en chat réel, `text` n'est
jamais `Never` après le premier message — **le balayage ne se
déclencherait plus jamais**. Les tuyauteries ne le voyaient pas : elles
parlent au backend sans chat, donc sans journal.

Correctif proposé (session embarquements, tenante de l'état) : un état
**par entité cible** — `index_state_for(entity)`, compté sur les tables
de l'entité, clé méta par entité — le global restant pour ce qui est
global. La décision de recherche (cible `Scope`) et la porte
d'`usages`/`impact` (l'entité du pivot) basculeront dessus. La reprise
T1 se rejouera après.

## T1 rejouée, après l'état par entité : la réponse est vraie

Avec les trois corrections en place (description d'`usages`, porte par
entité des deux côtés, journal sans effet sur l'état), l'agent faible
prend toujours `usages` — mais sa conclusion est désormais **vraie** :
« pas encore d'indexation, il faut indexer d'abord », avec l'appel
`index` proposé. Plus de « cette fonction n'existe pas ». L'outillage
corrige ce que le modèle ne sait pas rattraper.

Le décrochage multi-tour, lui, est confirmé comme **limite du modèle** :
le harnais renvoie `tools` à chaque tour (fiches relues fraîches,
`agent.rs`) et l'historique garde les `tool_calls` avec leurs `id` mot
pour mot (`openai_llm.rs`, testé en SSE) — et le 7B en 4 bits écrit
quand même son deuxième appel en bloc de code. Le rappel de protocole
proposé au document 03 reste la réponse si l'on veut faire mieux avec
les petits modèles.

C'est la leçon récurrente de la soirée : **chaque passe réelle a trouvé
un trou qu'aucun test scripté ne voyait** — l'outil muet sur un index
vide (faible), la traversée par la liste libre (Gemini), et l'état
global pollué par le journal (reprises). Le scénario rejouable est
l'outil de qualité le moins cher que nous ayons.
