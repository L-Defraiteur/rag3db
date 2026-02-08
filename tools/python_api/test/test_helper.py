import sys
from pathlib import Path

RAG3DB_ROOT = Path(__file__).parent.parent.parent.parent

if sys.platform == "win32":
    # \ in paths is not supported by rag3db's parser
    RAG3DB_ROOT = str(RAG3DB_ROOT).replace("\\", "/")
