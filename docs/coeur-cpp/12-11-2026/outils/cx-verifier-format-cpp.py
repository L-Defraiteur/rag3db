#!/usr/bin/env python3
"""Revue lexicale conservative du lot format18, pas preuve formelle de semantique C++."""
import collections
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
from pygments.lexers import CppLexer
from pygments.token import Comment, Text, Literal

ROOT = Path.cwd()
BASE = sys.argv[1]
lexer = CppLexer(ensurenl=False, stripnl=False)


def spelling(text):
    raw = list(lexer.get_tokens_unprocessed(text))
    hashes = {p + value.index('#') for p, typ, value in raw if typ in Comment.Preproc and '#' in value}
    region = 0
    includes = []
    kept = []
    offset = 0
    for line in text.splitlines(keepends=True):
        directive = re.match(r'^[ \t]*(#)[ \t]*(\w+)', line)
        is_directive = directive and offset + directive.start(1) in hashes
        inc = re.match(r'^[ \t]*#[ \t]*include[ \t]+([<"][^>"\n]*[>"])', line) if is_directive else None
        if inc:
            includes.append((region, inc[1]))
            kept.append('\n')
        else:
            if is_directive:
                region += 1
            kept.append(line)
        offset += len(line)
    tokens = []
    strings = []

    def flush():
        if not strings:
            return
        value = ''.join(strings)
        pos, prefixes, atoms = 0, set(), []
        while pos < len(value):
            match = re.match(r'(u8|u|U|L)?"((?:\\.|[^"\\])*)"', value[pos:], re.S)
            if not match:
                tokens.append(('literal_spelling', value))
                strings.clear()
                return
            if match[1]:
                prefixes.add(match[1])
            # Escapes remain spelling atoms, so splitting a hex/octal escape
            # across a literal boundary cannot silently compare equal.
            atoms.extend(re.findall(r'\\(?:x[0-9a-fA-F]+|u[0-9a-fA-F]{4}|U[0-9a-fA-F]{8}|[0-7]{1,3}|.)|[^\\]', match[2], re.S))
            pos += len(match[0])
        tokens.append(('adjacent_string_spellings', tuple(sorted(prefixes)), tuple(atoms)))
        strings.clear()

    for _, typ, value in lexer.get_tokens_unprocessed(''.join(kept)):
        if typ in Text and not value.strip():
            continue
        if typ in Comment and not (typ in Comment.Preproc or typ in Comment.PreprocFile):
            continue
        if typ in Literal.String:
            strings.append(value)
            continue
        flush()
        if typ in Comment.Preproc:
            value = re.sub(r'\s+', '', value)
        if value:
            tokens.append((str(typ), value))
    flush()
    # Only consecutive using declarations may have their order normalized.
    out = []
    i = 0
    while i < len(tokens):
        if tokens[i][-1] != 'using':
            out.append(tokens[i]); i += 1; continue
        declarations = []
        while i < len(tokens) and tokens[i][-1] == 'using':
            end = i
            while end < len(tokens) and tokens[end][-1] != ';':
                end += 1
            if end == len(tokens):
                break
            declarations.append(tuple(tokens[i:end + 1])); i = end + 1
        if not declarations:
            out.append(tokens[i]); i += 1
        else:
            out.append(('consecutive_using_declarations', tuple(sorted(declarations))))
    return collections.Counter(includes), out


changed = subprocess.check_output(['git', 'diff', '--name-only', BASE, '--', 'src', 'test', 'tools', 'extension'], text=True).splitlines()
results = []
for name in changed:
    old = subprocess.check_output(['git', 'show', BASE + ':' + name]).decode('utf-8')
    new = Path(name).read_text()
    a, b = spelling(old), spelling(new)
    results.append({'path': name, 'include_multiset_by_preprocessor_region_equal': a[0] == b[0],
                    'other_tokens_equal': a[1] == b[1],
                    'before_sha256': hashlib.sha256(old.encode()).hexdigest(),
                    'after_sha256': hashlib.sha256(new.encode()).hexdigest()})
report = {'base': BASE, 'changed_files': len(changed),
          'method': 'Pygments CppLexer; ignore whitespace/comments; include multiset within preprocessor regions; adjacent literal spelling atoms; consecutive using sort',
          'limit': 'not a compiler/AST proof; include-order changes still require private build and transaction tests',
          'files': results}
Path('docs/coeur-cpp/12-11-2026/preuves/ci-format18/revue-lexicale.json').write_text(json.dumps(report, indent=2) + '\n')
bad = [r for r in results if not r['include_multiset_by_preprocessor_region_equal'] or not r['other_tokens_equal']]
print(json.dumps({'changed': len(changed), 'different': bad}, indent=2))
sys.exit(1 if bad else 0)
