#!/usr/bin/env python3
"""One read-only unit query in the unchanged private validation environment."""
from pathlib import Path
import json,os,subprocess
root=Path.home()/'.cache/rag3weaver-build/bascule-live-20261116/service-guard-diagnostic-01';root.mkdir(mode=0o700,parents=True,exist_ok=False)
argv=['systemctl','--user','show','rag3weaver-daemon.service','--property=LoadState','--property=ActiveState']
result=subprocess.run(argv,capture_output=True,timeout=10)
(root/'stdout').write_bytes(result.stdout);(root/'stderr').write_bytes(result.stderr)
with (root/'observation.json').open('x') as stream:json.dump(dict(purpose='READ_ONLY_PRIVATE_ENVIRONMENT_OBSERVATION',argv=argv,rc=result.returncode,private_bus_address_set=bool(os.environ.get('DBUS_SESSION_BUS_ADDRESS')),xdg_runtime_dir=os.environ.get('XDG_RUNTIME_DIR'),units_active_qualified=False,writer_started=False),stream,indent=2)
