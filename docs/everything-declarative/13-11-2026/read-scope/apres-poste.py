"""Borne corrective héritée, calculée seulement APRÈS acquisition réelle."""
import importlib.util, json, math, os, pathlib, sys, time

ORIGINAL_DEADLINE = 908183.114381791


def main():
    os.chdir('/var/tmp')
    w, source, p, t = pathlib.Path(sys.argv[1]), sys.argv[2], pathlib.Path(sys.argv[3]), pathlib.Path(sys.argv[4])
    script = w / 'extension/rag3weaver/scripts/confiner_validation.py'
    spec = importlib.util.spec_from_file_location('confinement_ed', script)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    table = module.processus()
    assert module.poste_tenu(table), 'poste réel obligatoire avant calcul du restant'
    module.plafond(table[os.getpid()]['cgroup'])
    remaining = math.floor(ORIGINAL_DEADLINE - time.monotonic()) - 3
    assert remaining > 60, 'échéance R3 insuffisante ; aucun reset'
    (p / 'budget-restant.json').write_text(json.dumps(dict(original_deadline=ORIGINAL_DEADLINE, remaining=remaining, after_real_poste=True)))
    driver = w / 'docs/everything-declarative/13-11-2026/read-scope/driver.py'
    argv = [sys.executable, str(script), 'lancer', '--mode', 'lourd', '--delai', str(remaining),
            '--preuves', str(p / 'supervision'), '--', sys.executable, str(driver), str(w), source, str(p), str(t)]
    os.execve(sys.executable, argv, dict(os.environ))


if __name__ == '__main__':
    main()
