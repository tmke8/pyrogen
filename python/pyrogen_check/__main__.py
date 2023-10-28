import subprocess
import sys
import sysconfig
from pathlib import Path


def find_pyrogen_bin() -> Path:
    """Return the pyrogen binary path."""

    pyrogen_exe = "pyrogen" + sysconfig.get_config_var("EXE")

    path = Path(sysconfig.get_path("scripts")) / pyrogen_exe
    if path.is_file():
        return path

    user_scheme = sysconfig.get_preferred_scheme("user")

    path = Path(sysconfig.get_path("scripts", scheme=user_scheme)) / pyrogen_exe
    if path.is_file():
        return path

    raise FileNotFoundError(path)


if __name__ == "__main__":
    pyrogen = find_pyrogen_bin()
    completed_process = subprocess.run([pyrogen, *sys.argv[1:]])
    sys.exit(completed_process.returncode)
