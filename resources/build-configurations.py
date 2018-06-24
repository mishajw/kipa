#!/usr/bin/env python3

import io
import subprocess
from typing import List


def run_command(command: List[str]) -> bool:
    """
    Run a command, checking for errors/warnings and good exit code
    :param command: the command to check
    :return: success flag
    """

    success = True

    build_process = subprocess.Popen(
        command,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT)

    for line in io.TextIOWrapper(build_process.stdout, encoding="utf-8"):
        line = line.replace("\n", "")
        print(f"> {line}")
        if line.startswith("warning"):
            print("FATAL: Found warning")
            success = False
        if line.startswith("error"):
            print("FATAL: Found error")
            success = False

    build_process.wait()
    if build_process.returncode != 0:
        print(f"FATAL: Bad return code {build_process.returncode}")
        success = False

    return success


def build_and_test(args: List[str]) -> None:
    """
    Build the project and run tests with some arguments. Errors are raised if
    either fails
    :param args: the arguments passed to the build and check commands
    """

    print(f"Checking with flags: {args}")
    assert run_command(["cargo", "check"] + args), "Failed check"

    print(f"Testing with flags: {args}")
    assert run_command(["cargo", "test"] + args), "Failed test"


# Run default, i.e. no arguments
build_and_test([])

# Run different combinations of features
feature_sets = [
    # Run all configurations
    "", "use-graph use-protobuf use-tcp use-unix-socket", "use-black-hole"]
for fs in feature_sets:
    build_and_test(["--no-default-features", "--features", fs])
