#!/usr/bin/env python3
"""Deux retouches d'un graphe burn-onnx, réécrites en place, idempotentes :

1. chaque bloc QKᵀ → ÷√d → +masque → softmax → ·V devient l'attention
   fusionnée de burn (`burn::tensor::module::attention`, flash, accumulation
   f32) — s'il y en a ;
2. dans ces graphes-là seulement, chaque `.float().cast(DType::F32)` devient
   `.float()` : l'ONNX dit « float », pas « f32 », et sous Flex32 la fusion
   refuse le mélange avec les constantes du pack, elles aussi en Flex32.

Usage : patch_attention.py <fichier.rs> [...]
"""
import re, sys

def patcher(p):
    s = open(p).read()
    if "attention fusionnée" in s or "casts « float » neutres" in s:
        print("déjà patché"); return

    motif = re.compile(r'''
    (?P<indent>[ \t]*)let\ (?P<kt>\w+)\ =\ (?P<k_src>\w+)\.permute\(\[0,\ 2,\ 3,\ 1\]\);\n
    [ \t]*let\ (?P<scores>\w+)\ =\ (?P<q>\w+)\.matmul\((?P=kt)\);\n
    [ \t]*let\ (?P<cst>\w+)\ =\ self\.(?P<cst_field>\w+)\.val\(\);\n
    [ \t]*let\ (?P<div>\w+)\ =\ (?P=scores)\n
    [ \t]*\.div\(\((?P=cst)\)\.unsqueeze_dims\(&\[0isize,\ 1isize,\ 2isize\]\)\);\n
    [ \t]*let\ (?P<add>\w+)\ =\ (?P=div)\.add\((?P<mask>\w+)(?:\.clone\(\))?\);\n
    [ \t]*let\ (?P<soft>\w+)\ =\ burn::tensor::activation::softmax\((?P=add),\ 3\);\n
    [ \t]*let\ (?P<out>\w+)\ =\ (?P=soft)\.matmul\((?P<v>\w+)\);\n
    ''', re.VERBOSE)

    def remplace(m):
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

    s2, n = motif.subn(remplace, s)
    print(f"{n} blocs d'attention remplacés")

    # Les `Cast` vers F32 de l'ONNX veulent dire « float », pas « f32 ». Sous
    # Flex32 (stockage f32, matmul f16), `.float()` rend déjà le flottant de la
    # carte ; garder le `.cast(F32)` ré-étiquette f32 au milieu d'un graphe Flex32,
    # et la fusion, dont l'IR vérifie les dtypes, panique (`DTypeMismatch`).
    # …mais seulement là où l'attention a été fusionnée, c'est-à-dire BGE-M3 :
    # dans les autres graphes, la constante que le masque rencontre est
    # initialisée dans le code par burn-onnx (pas dans le pack), l'adaptateur
    # Flex32 ne la voit jamais, elle reste f32, et c'est le cast qui garde le
    # masque cohérent avec elle. Retirer le cast là-bas casse tout (6 sept.).
    m = 0
    if n > 0:
        s2, m = re.subn(r'\.float\(\)\.cast\(burn::tensor::DType::F32\)', '.float()', s2)
    print(f"{m} casts vers F32 retirés")
    if n == 0:
        print("rien à faire"); return
    open(p, 'w').write(s2)


for p in sys.argv[1:]:
    patcher(p)
