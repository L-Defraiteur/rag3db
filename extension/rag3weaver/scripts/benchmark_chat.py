#!/usr/bin/env python3
"""Run any configured chat application with a fixed prompt and retain its evidence.

No model launch, task vocabulary, tool dispatch or answer repair belongs here.
The same config can be used unchanged by chat_app.py (TUI + web).
"""
import argparse
import hashlib
import json
from pathlib import Path
import queue
import subprocess
import threading
import time
import uuid

ROOT = Path(__file__).resolve().parents[1]

def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("config", type=Path)
    p.add_argument("prompt", type=Path)
    p.add_argument("output", type=Path)
    p.add_argument("--binary", type=Path, default=ROOT / "target/debug/rag3weaver-chat")
    p.add_argument("--timeout", type=float, default=900)
    p.add_argument("--memory-path", type=Path, action="append", default=[])
    args = p.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    prompt = args.prompt.read_text()
    # Retain the exact inputs even if the working configuration changes later.
    (args.output / "config.json").write_bytes(args.config.read_bytes())
    (args.output / "prompt.txt").write_text(prompt)
    run_id = "bench-" + uuid.uuid4().hex
    report = {"session":run_id,"config":str(args.config.resolve()),"prompt_sha256":hashlib.sha256(prompt.encode()).hexdigest(),"generations":[],"tools":[],"memory":{},"first_content_seconds":None}
    samples, events = [], []
    stopped = threading.Event()
    def sample():
        while not stopped.is_set():
            row = {"time":time.time()}
            for path in args.memory_path:
                try: row[str(path)] = int(path.read_text().strip())
                except (OSError, ValueError): pass
            samples.append(row)
            stopped.wait(.2)
    thread = threading.Thread(target=sample, daemon=True)
    thread.start()
    with (args.output / "host.log").open("w") as err, (args.output / "events.jsonl").open("w") as evidence:
        process = subprocess.Popen([str(args.binary.resolve()),str(args.config.resolve())],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=err,text=True,bufsize=1)
        inbox = queue.Queue()
        def receive():
            for line in process.stdout: inbox.put(line)
            inbox.put(None)
        threading.Thread(target=receive,daemon=True).start()
        def send(value):
            process.stdin.write(json.dumps(value,ensure_ascii=False)+"\n")
            process.stdin.flush()
        def read_until_done(start):
            cancel_at = None
            while True:
                if cancel_at is None and time.monotonic()-start > args.timeout:
                    send({"op":"cancel"}); cancel_at=time.monotonic()
                    report["timeout_requested"] = True
                    print("Time budget reached; cooperative cancellation requested",flush=True)
                try: line=inbox.get(timeout=1)
                except queue.Empty:
                    if cancel_at and time.monotonic()-cancel_at>120:
                        raise TimeoutError("Provider did not acknowledge cancellation")
                    continue
                if line is None: raise RuntimeError("Host exited unexpectedly; see host.log")
                value=json.loads(line); elapsed=time.monotonic()-start
                row={"elapsed_seconds":elapsed,**value}; events.append(row)
                evidence.write(json.dumps(row,ensure_ascii=False)+"\n");evidence.flush()
                kind=value.get("event")
                if kind=="token" and report["first_content_seconds"] is None: report["first_content_seconds"]=elapsed
                if kind=="generation_end":
                    report["generations"].append(row)
                    print(f"generation: {value.get('completion_tokens','?')} tokens, {value.get('elapsed_ms',0)/1000:.1f}s",flush=True)
                if kind=="tool_start":
                    report["tools"].append(row)
                    print(f"tool {value['name']}: {value['arguments'][:180]}",flush=True)
                if kind=="done": return value
        try:
            start=time.monotonic();send({"op":"describe"})
            info=read_until_done(start);report["startup_seconds"]=time.monotonic()-start
            if not info.get("ok"): raise RuntimeError(info)
            report["application"]=info["result"]
            print("Application ready: "+info["result"]["name"],flush=True)
            start=time.monotonic();send({"op":"chat","session":run_id,"message":prompt})
            report["answer"]=read_until_done(start)
            report["task_seconds"]=time.monotonic()-start
            print(json.dumps(report["answer"],ensure_ascii=False),flush=True)
        except Exception as exc:
            report["error"]=str(exc)
            raise
        finally:
            stopped.set();thread.join()
            for path in args.memory_path:
                values=[r[str(path)] for r in samples if str(path) in r]
                if values: report["memory"][str(path)]={"baseline":values[0],"peak":max(values),"last":values[-1],"units":"as provided by file"}
            (args.output/"report.json").write_text(json.dumps(report,ensure_ascii=False,indent=2)+"\n")
            (args.output/"memory.jsonl").write_text("".join(json.dumps(r)+"\n" for r in samples))
            if process.poll() is None:
                send({"op":"shutdown"});process.stdin.close()
                # Let the backend checkpoint. Do not force-kill a database on timeout.
                process.wait()
            process.stdout.close()

if __name__=="__main__": main()
