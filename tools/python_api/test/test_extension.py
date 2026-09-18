from __future__ import annotations

import os
import platform
from pathlib import Path

import pytest
from type_aliases import ConnDB

EXTENSION_CMAKE_PREFIX = 'add_definitions(-DRAG3DB_EXTENSION_VERSION="'


@pytest.fixture
def extension_extension_dir_prefix() -> str:
    system = platform.system()
    extension_extension_dir_prefix = None
    if system == "Windows":
        extension_extension_dir_prefix = "win_amd64"
    elif system == "Linux":
        extension_extension_dir_prefix = (
            "linux_arm64" if platform.machine() == "aarch64" or platform.machine() == "arm64" else "linux_amd64"
        )
    elif system == "Darwin":
        extension_extension_dir_prefix = "osx_arm64" if platform.machine() == "arm64" else "osx_amd64"
    return extension_extension_dir_prefix
