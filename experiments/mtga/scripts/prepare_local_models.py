"""Fetch pinned public Bonsai artifacts into this experiment, preserving existing llama.cpp."""
import hashlib
import json
from pathlib import Path
import time
import urllib.request

ROOT = Path(__file__).resolve().parents[1] / "data/model-benchmark-2026-09-20"
ROOT.mkdir(parents=True, exist_ok=True)

def metadata(url):
    with urllib.request.urlopen(url, timeout=60) as response:
        return json.load(response)

def download(url, path, expected_size=None, expected_hash=None):
    if path.exists() and (expected_size is None or path.stat().st_size == expected_size):
        print(f"Reuse {path.name}", flush=True)
    else:
        partial = path.with_suffix(path.suffix + ".partial")
        start = time.monotonic()
        last = start
        with urllib.request.urlopen(url, timeout=120) as response, partial.open("wb") as out:
            total = int(response.headers.get("Content-Length", "0"))
            done = 0
            while chunk := response.read(4 * 1024 * 1024):
                out.write(chunk)
                done += len(chunk)
                now = time.monotonic()
                if now - last > 10:
                    print(f"{path.name}: {done/1e9:.2f}/{total/1e9:.2f} GB, {done/(now-start)/1e6:.1f} MB/s", flush=True)
                    last = now
        if expected_size and partial.stat().st_size != expected_size:
            raise RuntimeError("Download size mismatch")
        partial.rename(path)
    digest = hashlib.file_digest(path.open("rb"), "sha256").hexdigest()
    if expected_hash and digest != expected_hash.removeprefix("sha256:"):
        raise RuntimeError(f"Checksum mismatch: {path}")
    print(f"Verified {path.name}: {path.stat().st_size} bytes sha256={digest}", flush=True)
    return {"path": str(path), "bytes": path.stat().st_size, "sha256": digest, "url": url}

release = json.loads(Path("/tmp/bonsai-release.json").read_text())
asset = next(a for a in release["assets"] if a["name"].endswith("bin-ubuntu-rocm-7.2-x64.tar.gz"))
binary = download(asset["browser_download_url"], ROOT / asset["name"], asset["size"], asset.get("digest"))
repo = "prism-ml/Ternary-Bonsai-2-27B-gguf"
model = metadata("https://huggingface.co/api/models/" + repo + "?blobs=true")
filename = next(f for f in model["siblings"] if f["rfilename"].endswith("-PQ2_0.gguf"))
weights = download(f"https://huggingface.co/{repo}/resolve/{model['sha']}/{filename['rfilename']}", ROOT / filename["rfilename"], filename.get("size"), filename.get("lfs", {}).get("sha256"))
(ROOT / "downloads.json").write_text(json.dumps({"binary_release": release["tag_name"], "binary": binary, "model_revision": model["sha"], "model": weights}, indent=2) + "\n")
