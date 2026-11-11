#!/usr/bin/env python3
"""Complément borné : questions source figées, cibles A/B et oracle inchangés.

Les arêtes de la sonde portent une ligne mais pas le token de la mention.
Toutes les candidates de cette ligne sont conservées ; plusieurs arêtes
signalent une attribution à examiner, jamais une cible choisie par devinette.
Aucun score global de précision n'est calculé par ce complément.
"""
import argparse
import collections
import hashlib
import importlib.util
import json
import signal
from pathlib import Path
import time


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def load_manifest(path, root):
    data = json.loads(path.read_text())
    if data['schema_version'] != 1 or len(data['cases']) != 23:
        raise ValueError('liste bornée attendue : 23 questions, schéma 1')
    seen = set()
    for case in data['cases']:
        source = (root / case['source_file']).resolve(strict=True)
        if not source.is_relative_to(root) or sha(source) != case['source_sha256']:
            raise ValueError(f"source différente : {case['id']}")
        line = source.read_text().split('\n')[case['line_1based'] - 1]
        encoded = line.encode('utf-16-le')
        start = case['column_utf16_0based'] * 2
        size = len(case['identifier'].encode('utf-16-le'))
        if encoded[start:start + size].decode('utf-16-le') != case['identifier']:
            raise ValueError(f"position/token différent : {case['id']}")
        key = (str(source), case['line_1based'], case['column_utf16_0based'])
        if key in seen:
            raise ValueError(f"question dupliquée : {case['id']}")
        seen.add(key)
    return data


def query(server, oracle, source, case):
    calls = []
    positions = []
    for attempt in range(3):
        for method in ('textDocument/definition', 'textDocument/declaration'):
            params = {'textDocument': {'uri': f'file://{source}'},
                      'position': {'line': case['line_1based'] - 1,
                                   'character': case['column_utf16_0based']}}
            response = server.appel(method, params)
            calls.append({'attempt': attempt + 1, 'method': method, 'params': params,
                          'response': response})
            result = response.get('result') if response else None
            if isinstance(result, dict):
                result = [result]
            for item in result or []:
                uri = item.get('uri') or item.get('targetUri')
                span = item.get('range') or item.get('targetSelectionRange') or item.get('targetRange')
                if uri and span:
                    positions.append((oracle.ramener(uri.replace('file://', '')),
                                      span['start']['line'] + 1))
        if positions:
            break
        if attempt < 2:
            time.sleep(2)
    return calls, sorted(set(positions))


def main():
    def terminate(signum, _frame):
        raise SystemExit(128 + signum)
    signal.signal(signal.SIGTERM, terminate)
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('manifest', 'root', 'measurement', 'oracle', 'output'):
        parser.add_argument(name, type=Path)
    args = parser.parse_args()
    root = args.root.resolve(strict=True)
    manifest = load_manifest(args.manifest, root)
    if sha(args.oracle) != manifest['historical_oracle_sha256']:
        raise ValueError('oracle historique différent de la preuve figée')
    resume = (args.measurement / 'resume.txt').read_text()
    if not resume.rstrip().endswith('fini'):
        raise ValueError('le banc doit être entièrement terminé avant ce complément LSP')
    for state, key in [('A', 'reference_revision'), ('B', 'candidate_revision')]:
        if f"{state} : arbre {manifest[key][:9]}," not in resume:
            raise ValueError(f'hash de mesure différent : {state}')
    spec = importlib.util.spec_from_file_location('oracle_historique', args.oracle)
    oracle = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(oracle)
    oracle.RACINE[0] = str(root)
    selected = {(c['source_file'], c['line_1based']) for c in manifest['cases']}
    edges = {}
    for state in ('A', 'B'):
        all_edges = oracle.lire_aretes(args.measurement / f'aretes-{state}.txt')
        if not all_edges:
            raise ValueError(f'sonde sans arêtes : {state}')
        grouped = collections.defaultdict(list)
        for edge in all_edges:
            if (edge['de'], edge['ligne']) in selected:
                grouped[(edge['de'], edge['ligne'])].append(edge)
        edges[state] = grouped
    args.output.mkdir(parents=True, exist_ok=False)
    metadata = {'completed': False, 'manifest_sha256': sha(args.manifest),
                'oracle_sha256': sha(args.oracle), 'runner_sha256': sha(Path(__file__)),
                'measurement': str(args.measurement), 'resume_sha256': sha(args.measurement / 'resume.txt'),
                'edge_sha256': {s: sha(args.measurement / f'aretes-{s}.txt') for s in ('A', 'B')},
                'manifest': manifest, 'protocol': 'same 23 source questions; shared answers for both states; up to 3 attempts only when no position; strict predicate unchanged; all edges on source line preserved'}
    meta_path = args.output / 'provenance.json'
    meta_path.write_text(json.dumps(metadata, ensure_ascii=False, indent=2) + '\n')
    command, languages, options, _ = oracle.SERVEURS['typescript']
    metadata['lsp_command'] = command
    metadata['lsp_initialization_options'] = options
    # Keep ownership even if initialization raises after starting the child.
    server = oracle.Serveur.__new__(oracle.Serveur)
    try:
        server.__init__(command, str(root), options)
        opened = {str(root / c['source_file']) for c in manifest['cases']}
        for grouped in edges.values():
            opened.update(str(root / edge['vers']) for values in grouped.values() for edge in values
                          if (root / edge['vers']).is_file())
        metadata['opened_files_sha256'] = {}
        for file in sorted(opened):
            path = Path(file).resolve(strict=True)
            if not path.is_relative_to(root):
                raise ValueError(f'fichier à ouvrir hors corpus : {path}')
            metadata['opened_files_sha256'][str(path.relative_to(root))] = sha(path)
            server.ouvrir(str(path), oracle.langue_de(languages, str(path)))
        ts_path = options.get('tsserver', {}).get('path')
        if ts_path:
            ts_script = Path(ts_path)
            if ts_script.is_dir():
                ts_script = ts_script / 'tsserver.js'
            if ts_script.is_file():
                metadata['tsserver_script_sha256'] = sha(ts_script)
        meta_path.write_text(json.dumps(metadata, ensure_ascii=False, indent=2) + '\n')
        metadata['initialization_wait_seconds'] = server.attendre_le_repos(plafond=600, exiger=False)
        with (args.output / 'questions.jsonl').open('w') as questions, \
             (args.output / 'A.jsonl').open('w') as output_a, \
             (args.output / 'B.jsonl').open('w') as output_b:
            for case in manifest['cases']:
                calls, positions = query(server, oracle, root / case['source_file'], case)
                rpc_errors = sum(c['response'] is None or 'error' in c['response'] for c in calls)
                raw = {'case': case, 'rpc': calls, 'positions': positions,
                       'question_state': ('lsp_avec_positions' if positions else
                                          'rpc_erreur_ou_delai' if rpc_errors else 'lsp_interroge_sans_position'),
                       'rpc_errors_or_timeouts': rpc_errors,
                       'positions_outside_root': [p for p in positions if not Path(p[0]).resolve().is_relative_to(root)]}
                questions.write(json.dumps(raw, ensure_ascii=False) + '\n')
                questions.flush()
                for state, output in [('A', output_a), ('B', output_b)]:
                    candidates = edges[state][(case['source_file'], case['line_1based'])]
                    judgements = [{'edge': edge,
                                   'strict_verdict': ('juste' if oracle.dans_la_cible(positions, edge, str(root))
                                                      else 'faux' if positions else 'non_evalue_sans_position')}
                                  for edge in candidates]
                    record = {'case_id': case['id'], 'source_question': {k: case[k] for k in
                              ('source_file', 'line_1based', 'column_utf16_0based', 'identifier')},
                              'attribution': ('absence_arete' if not candidates else 'unique_arete_sur_ligne'
                                              if len(candidates) == 1 else 'plusieurs_aretes_sur_ligne_a_examiner'),
                              'strict_judgements': judgements}
                    output.write(json.dumps(record, ensure_ascii=False) + '\n')
                    output.flush()
        metadata['completed'] = True
    finally:
        try:
            server.appel('shutdown', None, delai=10)
            server.notifier('exit', None)
            server.p.wait(timeout=10)
        except Exception:
            if hasattr(server, 'p'):
                server.p.terminate()
                try:
                    server.p.wait(timeout=10)
                except Exception:
                    server.p.kill()
                    server.p.wait(timeout=10)
        meta_path.write_text(json.dumps(metadata, ensure_ascii=False, indent=2) + '\n')
    print(json.dumps({'completed': True, 'questions': len(manifest['cases']), 'output': str(args.output)}, ensure_ascii=False))


if __name__ == '__main__':
    main()
