import logging
import os
import shutil
import subprocess
import tempfile
from pathlib import Path
from typing import List, NamedTuple, Dict, FrozenSet

from simulation import Build
from simulation.networks import Node, NodeId

log = logging.getLogger(__name__)


class BuildArgs(NamedTuple):
    additional_features: FrozenSet[str]
    clear_default_features: bool
    debug: bool


def create_builds(nodes: List[Node]) -> Dict[NodeId, Build]:
    node_to_args = dict(
        (
            node.id,
            BuildArgs(
                node.additional_features,
                node.clear_default_features,
                node.debug,
            ),
        )
        for node in nodes
    )

    build_args = set(node_to_args.values())
    builds_to_directories = dict(
        (args, __create_build(args)) for args in build_args
    )

    return {
        node_id: builds_to_directories[args]
        for node_id, args in node_to_args.items()
    }


def __create_build(args: BuildArgs) -> Build:
    directory = Path(tempfile.mkdtemp())
    log.debug(f"Made build directory at {directory}")

    build_command = ["cargo", "build"]
    if not args.debug:
        build_command += ["--release"]
    if args.clear_default_features:
        build_command += ["--no-default-features"]
    if args.additional_features:
        build_command += ["--features", " ".join(args.additional_features)]

    log.debug(f"Building with command {build_command}")
    build_process = subprocess.Popen(build_command)
    build_process.wait()
    assert build_process.returncode == 0, "KIPA build command failed"

    if args.debug:
        binary_directory = Path("target/debug")
    else:
        binary_directory = Path("target/release")

    log.debug("Extracting cli binary")
    cli_path = binary_directory / "kipa"
    assert os.path.isfile(cli_path)
    shutil.copyfile(cli_path, directory / "kipa")

    log.debug("Extracting daemon binary")
    daemon_path = binary_directory / "kipa-daemon"
    assert os.path.isfile(daemon_path)
    shutil.copyfile(daemon_path, directory / "kipa-daemon")

    return Build(cli_path, daemon_path)
