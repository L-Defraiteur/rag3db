# Tentative 1 des fiches classées — 11 octobre 2026, 18 h

*Le sujet d'une fiche au `put_memory`, décidé oui / non sur chaque candidat. Régime de
Lucie : on tente avec les briques qu'on a, on mesure, on ajuste ; une tentative ne se
réécrit pas, la suivante est un nouveau dossier.*

## Briques posées

- le banc : `tests/e2e_banc_fiches_classees.rs`, jeu `tests/data/banc-fiches-classees/v1.json`
  (10 sujets, 39 fiches dont 8 sans sujet) ;
- le gabarit `memory` tel qu'il est livré (`put_subject`, `recall_subjects`) ;
- le nœud `Decide` du proto (7b5cda799), avec son paramètre `prompt` (a6bfb9b9a) ;
- le Decider à un jeton sur un llama-server local, Qwen3-1.7B Q8_0 (Apache-2.0,
  `~/models/qwen3-1.7b/`) ; la sortie structurée par Gemini (Vertex) pour juger.

## Assemblage

Pour chaque fiche : `recall_subjects(claim, limit 3)` donne les candidats ; pour chaque
candidat, le graphe de la tentative (`decide_subject_decider.mmd` ou `_llm.mmd`) reçoit
`{text: "Sujet : … Fiche : … Pourquoi : … Cette fiche va-t-elle sous ce sujet ?"}` et
rend `{verdict: yes|no, verdict_probability}`. Une fiche est juste si l'ensemble de ses
« yes » est exactement son sujet (vide pour une fiche sans sujet).

## Ce qui a tourné

- **Le rappel des candidats** : 29 / 31 fiches à sujet ont leur sujet parmi les trois
  candidats (3,4 s pour 39 recherches). Deux fiches sont perdues avant toute décision :
  le plafond du classement est à 29 / 31.
- **La voie du Decider : rouge à la première décision.** `NoOptionInSight` — le modèle
  voyait comme jetons suivants `` `\n` ``, `` `\\n` ``, `Okay`, `Je`… : le gabarit ChatML
  n'était pas formé.
- La voie de Gemini n'a pas été jouée dans cette tentative.

## Coût mesuré

- llama-server (Qwen3-1.7B Q8_0, contexte 8192) : **+2 859 Mio** de mémoire GPU partagée
  (GTT de l'iGPU), prêt en moins de 2 s, 670 Mio de mémoire vive.

## Le retour

**Une valeur entre apostrophes d'un graphe mermaid ne porte pas d'échappement** :
`prompt='<|im_start|>system\n…'` passe le `\n` tel quel, deux caractères
(`parse_value`, `src/dataflow/mermaid.rs`). Le gabarit ChatML, qui vit de ses retours à la
ligne, ne peut pas s'écrire dans un `.mmd`. Ajouter les échappements à `parse_mermaid`
toucherait tous les graphes existants (des scripts rhai, des motifs) : c'est au proto d'en
juger, signalé.

## Ce qu'on ajuste

La tentative 2 (`2026-10-11-fiches-classees-2/`) porte le graphe du Decider en définition
JSON (`decide_subject_decider.json`), où un retour à la ligne est un retour à la ligne ; le
reste est inchangé.
