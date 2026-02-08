import os
import sys
from enum import Enum
from pathlib import Path

RAG3DB_ROOT = Path(__file__).parent.parent.parent.parent
if sys.platform == "win32":
    # \ in paths is not supported by rag3db's parser
    RAG3DB_ROOT = str(RAG3DB_ROOT).replace("\\", "/")

RAG3DB_EXEC_PATH = os.path.join(
    RAG3DB_ROOT,
    "build",
    "release",
    "tools",
    "shell",
    "rag3db",
)


def _get_rag3db_version():
    cmake_file = os.path.join(RAG3DB_ROOT, "CMakeLists.txt")
    with open(cmake_file) as f:
        for line in f:
            if line.startswith("project(Rag3db VERSION"):
                return line.split(" ")[2].strip()
        return None


RAG3DB_VERSION = _get_rag3db_version()


class KEY_ACTION(Enum):
    KEY_NULL = "\0"  # NULL
    CTRL_A = "a"  # Ctrl-a
    CTRL_B = "b"  # Ctrl-b
    CTRL_C = "c"  # Ctrl-c
    CTRL_D = "d"  # Ctrl-d
    CTRL_E = "e"  # Ctrl-e
    CTRL_F = "f"  # Ctrl-f
    CTRL_G = "g"  # Ctrl-g
    CTRL_H = chr(8)  # Ctrl-h
    TAB = "\t"  # Tab
    CTRL_K = "k"  # Ctrl-k
    CTRL_L = "l"  # Ctrl-l
    ENTER = "\r"  # Enter
    CTRL_N = "n"  # Ctrl-n
    CTRL_P = "p"  # Ctrl-p
    CTRL_R = "r"  # Ctrl-r
    CTRL_S = "s"  # Ctrl-s
    CTRL_T = "t"  # Ctrl-t
    CTRL_U = "u"  # Ctrl-u
    CTRL_W = chr(23)  # Ctrl-w
    ESC = "\27"  # Escape
    BACKSPACE = chr(127)  # Backspace


def deleteIfExists(file) -> None:
    if os.path.exists(file):
        os.remove(file)
