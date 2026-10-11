# Tentative 2 des fiches classées — 11 octobre 2026, 18 h 30

*Même assemblage que la tentative 1 ; un seul ajustement : le graphe du Decider en
définition JSON (`decide_subject_decider.json`), parce qu'une valeur mermaid ne porte pas
de retour à la ligne (retour de la tentative 1).*

## Ce qui a tourné

| | voie du Decider (Qwen3-1.7B local) | voie du LLM (Gemini 3.5 Flash, Vertex) |
|---|---|---|
| rappel des candidats | 29 / 31 | 29 / 31 |
| fiches justes | **25 / 39** | — (rouge, voir plus bas) |
| abstentions justes (fiches sans sujet) | **8 / 8** | — |
| ancrages justes / **faux** / manqués | 18 / **3** / 11 | — |
| décisions, temps | 117 en 7,3 s, **62 ms** l'une | ~8,5 min avant la chute, **~5 s** l'une |
| mémoire GPU | +2 859 Mio (GTT, llama-server) | aucune ici |

`resultats/decider-qwen3-1.7b.json` garde chaque décision (son champ `proba_oui` est
la probabilité de l'étiquette **choisie** — mal nommé, renommé `proba_verdict` dans le
banc depuis) ; `resultats/llm-gemini.log` la chute.

**La voie du Decider fait son métier de témoin du protocole** : chaque décision rend une
étiquette de la liste, et le graphe de la tentative tourne de bout en bout. Ce n'est pas
un juge : sur 11 ancrages manqués et 3 faux, le modèle est **sûr de lui** — la probabilité
de l'étiquette choisie vaut 0,92 à 1,0 dans les manqués, 0,96 à 1,0 dans les faux
(« un e2e sans ignore » rangé sous « régime GPU » à 0,965). Un seuil d'abstention sur
cette probabilité n'y changerait rien : c'est la mesure du 10 octobre (« un score ordonne
bien et juge mal ») retrouvée.

**La voie de Gemini est rouge** à une décision : la réponse est arrivée coupée,
`{"label": "no`, et `Decide` l'a refusée comme « étiquette hors liste ». Cause lue dans
`decide_nodes.rs` (`by_llm`) : le plafond par défaut de 512 jetons, que la réflexion de
Gemini 3.5 Flash consomme, et la raison de fin de génération (`Finish`) qui n'est pas lue.
Le jugement de Gemini n'a donc pas été obtenu.

## Le retour

1. `Decide` sur la voie du LLM a besoin d'un **plafond de jetons réglable** (ou d'un
   plafond qui laisse place à la réflexion), et doit **lire la raison de fin** : une
   réponse tronquée se dit « tronquée », pas « hors liste ». Signalé au proto.
2. Le coût de Gemini par décision (~5 s, réflexion comprise) fait d'un oui / non par
   candidat une dépense : 117 décisions pour 39 fiches. À mesurer contre une décision par
   fiche (« lequel des candidats, ou aucun »), qui est la forme que la sortie structurée
   permet et le Decider non.

## Ce qu'on ajuste

La tentative 3 rejoue Gemini quand `Decide` aura son plafond ; d'ici là, rien à
assembler autrement de ce côté.
