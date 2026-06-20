import os
import sys
import sysconfig


def find_vyp_bin() -> str:
    """Return the path to the vyp binary."""
    scripts = sysconfig.get_path("scripts")
    if scripts:
        binary = os.path.join(scripts, "vyp")
        if os.path.isfile(binary):
            return binary
    raise FileNotFoundError("vyp binary not found")


def main():
    vyp = find_vyp_bin()
    os.execvp(vyp, [vyp] + sys.argv[1:])
