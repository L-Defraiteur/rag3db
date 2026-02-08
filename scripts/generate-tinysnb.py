import os
import shutil
import subprocess
import sys

RAG3DB_ROOT = os.path.dirname(os.path.dirname(os.path.realpath(__file__)))
# Datasets can only be copied from the root since copy.schema contains relative paths
os.chdir(RAG3DB_ROOT)

# Define the build type from input
if len(sys.argv) > 1 and sys.argv[1].lower() == "release":
    build_type = "release"
else:
    build_type = "relwithdebinfo"

# Change the current working directory
if os.path.exists(f"{RAG3DB_ROOT}/dataset/databases/tinysnb"):
    shutil.rmtree(f"{RAG3DB_ROOT}/dataset/databases/tinysnb")
if sys.platform == "win32":
    rag3db_shell_path = f"{RAG3DB_ROOT}/build/{build_type}/src/rag3db_shell"
else:
    rag3db_shell_path = f"{RAG3DB_ROOT}/build/{build_type}/tools/shell/rag3db"
subprocess.check_call(
    [
        "python3",
        f"{RAG3DB_ROOT}/benchmark/serializer.py",
        "TinySNB",
        f"{RAG3DB_ROOT}/dataset/tinysnb",
        f"{RAG3DB_ROOT}/dataset/databases/tinysnb",
        "--single-thread",
        "--rag3db-shell",
        rag3db_shell_path,
    ]
)
