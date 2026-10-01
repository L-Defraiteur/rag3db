#!/usr/bin/env python3
"""Measure a configured local server at several parameter values; argv only, no shell.

Use {value} in command arguments. Readiness URL and memory counter files come
from JSON config. No model, hardware, provider or application-specific branches.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess
import time
import urllib.error
import urllib.request

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("config",type=Path)
    parser.add_argument("output",type=Path)
    args=parser.parse_args(); config=json.loads(args.config.read_text())
    args.output.mkdir(parents=True,exist_ok=True)
    reports=[]
    def memory():
        return {path:int(Path(path).read_text()) for path in config["memory_files"]}
    for value in config["values"]:
        report={"value":value,"baseline":memory(),"ready":False}
        command=[arg.replace("{value}",str(value)) for arg in config["command"]]
        report["command"]=command; start=time.monotonic()
        with (args.output/f"server-{value}.log").open("w") as log:
            process=subprocess.Popen(command,env={**os.environ,**config.get("env",{})},stdout=log,stderr=subprocess.STDOUT)
            try:
                while time.monotonic()-start<config.get("startup_timeout_seconds",120):
                    if process.poll() is not None: raise RuntimeError(f"server exit {process.returncode}")
                    used=memory()
                    report["peak"]={k:max(v,report.get("peak",{}).get(k,0)) for k,v in used.items()}
                    if any(used[k]>limit for k,limit in config.get("ceilings",{}).items()):
                        raise RuntimeError("configured memory ceiling exceeded")
                    try:
                        with urllib.request.urlopen(config["ready_url"],timeout=1) as response:
                            if response.status==200: report["ready"]=True; break
                    except (urllib.error.URLError,TimeoutError): pass
                    time.sleep(.1)
                if not report["ready"]: raise TimeoutError("server readiness timeout")
                report["startup_seconds"]=time.monotonic()-start
                if "probe" in config:
                    probe=config["probe"]
                    req=urllib.request.Request(probe["url"],data=json.dumps(probe["body"]).encode(),headers={"Content-Type":"application/json"})
                    with urllib.request.urlopen(req,timeout=60) as response: report["probe"]=json.load(response)
                report["loaded"]=memory()
                print(json.dumps(report),flush=True)
            except Exception as exc:
                report["error"]=str(exc); print(json.dumps(report),flush=True)
            finally:
                if process.poll() is None: process.terminate()
                process.wait(timeout=60)
                time.sleep(.3)
                report["after_stop"]=memory();reports.append(report)
                (args.output/"report.json").write_text(json.dumps(reports,indent=2)+"\n")

if __name__=="__main__": main()
