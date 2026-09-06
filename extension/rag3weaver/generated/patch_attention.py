#!/usr/bin/env python3
"""Trois retouches d'un graphe burn-onnx, réécrites en place, idempotentes :

1. chaque bloc QKᵀ → ÷√d → +masque → softmax → ·V (burn-onnx pre.1, BGE-M3)
   devient l'attention fusionnée de burn (`burn::tensor::module::attention`,
   flash, accumulation f32) — s'il y en a ;
2. là où burn-onnx pre.3 émet déjà `module::attention` (Granite), le masque
   passe de biais additif (`attn_bias`) à booléen (`mask`) : avec un biais,
   burn-cubecl retombe sur l'attention naïve et matérialise les scores
   (3,2 Go pour 256 séquences de 512) ; avec un masque, la flash attention prend ;
3. chaque `.float().cast(DType::F32)` devient `.float()` : l'ONNX dit « float »,
   pas « f32 », et sous Flex32 la fusion refuse le mélange avec les constantes
   du pack, elles aussi en Flex32.

Usage : patch_attention.py [--casts-neutres] <fichier.rs> [...]
  La règle 3 s'applique quand la 1 ou la 2 a joué, ou sur `--casts-neutres`.
  Elle dépend de la façon dont l'embarqueur CHARGE le graphe, pas du graphe :
  BGE-M3 et Granite passent par BurnpackStore + Flex32Adapter, la constante du
  masque arrive donc en Flex32 et le cast f32 du masque casse la fusion ;
  MiniLM et les rerankers passent aussi par le store depuis le 6 septembre au
  soir (`--casts-neutres` chez eux aussi).
"""
import re
import sys

MARQUE = "// rag3weaver : patch_attention.py"

NAIF = re.compile(r'''
(?P<indent>[ \t]*)let\ (?P<kt>\w+)\ =\ (?P<k_src>\w+)\.permute\(\[0,\ 2,\ 3,\ 1\]\);\n
[ \t]*let\ (?P<scores>\w+)\ =\ (?P<q>\w+)\.matmul\((?P=kt)\);\n
[ \t]*let\ (?P<cst>\w+)\ =\ self\.(?P<cst_field>\w+)\.val\(\);\n
[ \t]*let\ (?P<div>\w+)\ =\ (?P=scores)\n
[ \t]*\.div\(\((?P=cst)\)\.unsqueeze_dims\(&\[0isize,\ 1isize,\ 2isize\]\)\);\n
[ \t]*let\ (?P<add>\w+)\ =\ (?P=div)\.add\((?P<mask>\w+)(?:\.clone\(\))?\);\n
[ \t]*let\ (?P<soft>\w+)\ =\ burn::tensor::activation::softmax\((?P=add),\ 3\);\n
[ \t]*let\ (?P<out>\w+)\ =\ (?P=soft)\.matmul\((?P<v>\w+)\);\n
''', re.VERBOSE)

BIAIS = re.compile(r'''
(?P<indent>[ \t]*)let\ (?P<out>\w+)\ =\ burn::tensor::module::attention\(\n
[ \t]*q,\n
[ \t]*k,\n
[ \t]*v,\n
[ \t]*None,\n
[ \t]*Some\((?P<mask>\w+)(?:\.clone\(\))?\),\n
''', re.VERBOSE)


def fusionner(m):
    i = m.group('indent')
    return (
        f"{i}// rag3weaver : attention fusionnée (flash, accumulation f32) à la place de\n"
        f"{i}// QKᵀ/÷√d/+masque/softmax/·V — patch_attention.py, 6 septembre 2026.\n"
        f"{i}// L'échelle par défaut de `attention` est 1/√d, ce que faisait `{m.group('cst_field')}`.\n"
        f"{i}let {m.group('kt')} = {m.group('k_src')}.permute([0, 2, 1, 3]);\n"
        f"{i}let _ = self.{m.group('cst_field')}.val();\n"
        f"{i}let {m.group('out')} = {{\n"
        f"{i}    let [b, h, sq, _] = {m.group('q')}.dims();\n"
        f"{i}    let sk = {m.group('kt')}.dims()[2];\n"
        f"{i}    let masque = {m.group('mask')}.clone().lower_elem(0.0).expand([b, h, sq, sk]);\n"
        f"{i}    burn::tensor::module::attention(\n"
        f"{i}        {m.group('q')},\n"
        f"{i}        {m.group('kt')},\n"
        f"{i}        {m.group('v')},\n"
        f"{i}        Some(masque),\n"
        f"{i}        None,\n"
        f"{i}        burn::tensor::ops::AttentionModuleOptions::default(),\n"
        f"{i}    )\n"
        f"{i}}};\n"
    )


def masque_booleen(m):
    i = m.group('indent')
    return (
        f"{i}// rag3weaver : le masque en booléen, pas en biais — avec un biais,\n"
        f"{i}// burn-cubecl retombe sur l'attention naïve (patch_attention.py, 6 sept. 2026).\n"
        f"{i}let masque = {{\n"
        f"{i}    let [b, h, sq, _] = q.dims();\n"
        f"{i}    let sk = k.dims()[2];\n"
        f"{i}    {m.group('mask')}.clone().lower_elem(0.0).expand([b, h, sq, sk])\n"
        f"{i}}};\n"
        f"{i}let {m.group('out')} = burn::tensor::module::attention(\n"
        f"{i}    q,\n"
        f"{i}    k,\n"
        f"{i}    v,\n"
        f"{i}    Some(masque),\n"
        f"{i}    None,\n"
    )


def patcher(p, casts_neutres):
    s = open(p).read()
    s2, n = NAIF.subn(fusionner, s)
    s2, b = BIAIS.subn(masque_booleen, s2)
    m = 0
    if n > 0 or b > 0 or casts_neutres:
        s2, m = re.subn(r'\.float\(\)\.cast\(burn::tensor::DType::F32\)', '.float()', s2)
    print(f"{p} : {n} blocs naïfs fusionnés, {b} masques passés de biais à booléen, {m} casts vers F32 retirés")
    if n == 0 and b == 0 and m == 0:
        return
    if MARQUE not in s2:
        s2 = f"{MARQUE} — attention fusionnée / masque booléen / casts « float » neutres, 6 septembre 2026.\n" + s2
    open(p, 'w').write(s2)


if __name__ == "__main__":
    casts_neutres = "--casts-neutres" in sys.argv
    for p in [a for a in sys.argv[1:] if not a.startswith("--")]:
        patcher(p, casts_neutres)
