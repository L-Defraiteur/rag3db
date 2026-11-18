# Les paramètres-propriétés TypeScript manquent aux membres

- **État** : ouvert ; baseline observée, aucun correctif ni rejeu.
- **Pour** : Codeparsers après END de baseline.
- **Famille** : produit ; membres de classe absents sur AST valide.
- **Parent** : docs/codeparsers/tickets/2026-11-16-verifier-la-couverture-sur-des-corpus-publics-multilangages.md.

Source gel02 : vscode6e68549edb69380c9316e49df5d3795d5064a2b0,
`src/vs/editor/common/core/2d/point.ts`, SHA256
df5618042c1771f60d6ba34dd77ce1caa193d22eaee462cbdb0fb3933146693a.
Le constructeur lignes11–14 déclare `public readonly x: number` et y : en
TypeScript, ce sont des paramètres ET des propriétés de la classe Point.
Les deux faits ont été écrits dans l'oracle905bf17f36 avant lecture des sorties.

ELFbff1cafb...5565e2, ASTvalid/completed, compact_inventory_omitted=0 : les11
méthodes et le propriétaire Point sont présents, paramètres x/y type number
correctement conservés aux lignes12/13 sous constructor. member_facts=11 et
l'inventaire exhaustif ne contient que ces méthodes, sans propriétés x/y.
C'est l'absence des membres qui est confirmée, pas l'absence de paramètres.

ClassMemberInfo expose déjà Property et is_readonly : la représentation commune
peut porter ces faits. Le compact ne conserve pas is_readonly : aucune absence
de ce drapeau dans l'IR n'est déduite de la seule sortie. Le futur témoin doit
vérifier explicitement membres ET drapeaux sur l'IR, puis la preuve compacte
correspondante ; ne pas élargir le ticket à toute l'analyse des modifiers.

Minimal proposé, **non joué** :

```ts
class Point {
  constructor(public readonly x: number, public readonly y: number) {}
}
```

Rouge avant fix : deux membres Property owner Point, type number, readonly,
plus paramètres du constructeur. Un paramètre ordinaire ne doit pas devenir
une propriété ; couvrir ce négatif. Aucun correctif avant END de baseline.

Preuve figée : oracle-review-02/sample-results.jsonl SHA256
3e3ee90db2111430e8bb9f6e27db6bccc92df673346574df7804a0375fc311ff,
sous `~/.cache/rag3db-banc-notes/public-corpus-20261116/`, aux deux postes.
Pas de reparse, build externe, modèle, service ou DB dans ce diagnostic.

## Relu le 18 novembre (session codeparsers)

Non rejoué. Le constat est précis et le témoin minimal est bon ; à jouer avant toute correction.

Le parent `2026-11-16-verifier-la-couverture-sur-des-corpus-publics-multilangages.md`, le gel et les jugements restent sur la branche `cx-banc-corpus-public-20261116` ; le banc décide s'il les pose.
