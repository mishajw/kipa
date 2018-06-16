"""
Code for creating networks of docker containers.
"""

import logging
import os
import shutil
import subprocess
import tempfile
from typing import Iterator, List

import docker

from benchmarks.networks import Node
from benchmarks.key_creator import create_keys, GPG_HOME
from benchmarks.networks import Network

log = logging.getLogger(__name__)

DOCKER_PREFIX = "kipa_benchmark"
IMAGE_NAME = DOCKER_PREFIX
NETWORK_NAME = f"{DOCKER_PREFIX}_network"
IPV4_PREFIX = "192.168.123"


def create(size: int, daemon_args: str) -> Network:
    """Create a network of the specified size"""

    key_ids = create_keys(size)

    log.info("Getting docker client")
    client = docker.from_env()

    log.info("Creating docker directory")
    docker_directory = __create_docker_directory()

    log.info("Building KIPA image (may take a while)")
    client.images.build(
        path=docker_directory,
        tag=IMAGE_NAME,
        quiet=False)

    log.info("Removing docker directory")
    shutil.rmtree(docker_directory)

    log.info("Deleting old docker constructs")
    __delete_old(client)

    log.info("Building a network for containers")
    network = client.networks.create(
        NETWORK_NAME,
        driver="bridge",
        ipam=docker.types.IPAMConfig(
            pool_configs=[docker.types.IPAMPool(
                subnet=f"{IPV4_PREFIX}.0/24",
                gateway=f"{IPV4_PREFIX}.123")]))

    log.info(f"Creating {len(key_ids)} containers")
    containers = list(__create_nodes(client, key_ids, daemon_args, network))
    return Network(containers)


def __create_docker_directory() -> str:
    docker_directory = tempfile.mkdtemp()

    log.debug(f"Made docker directory at {docker_directory}")

    build_process = subprocess.Popen(["cargo", "build", "--release"])
    build_process.wait()
    assert build_process.returncode == 0, "KIPA build command failed"

    assert os.path.isfile("target/release/kipa_daemon")
    shutil.copyfile(
        "target/release/kipa_daemon",
        os.path.join(docker_directory, "kipa_daemon"))

    assert os.path.isfile("target/release/kipa_cli")
    shutil.copyfile(
        "target/release/kipa_cli",
        os.path.join(docker_directory, "kipa_cli"))

    with open(os.path.join(docker_directory, "Dockerfile"), "w") as f:
        # TODO: Base docker image has to use the same `glibc` as host machine
        f.write(f"""
            FROM debian:buster-slim
            ENV KIPA_KEY_ID ""
            ENV KIPA_ARGS ""
            RUN \\
                apt-get update && apt-get --yes install gpg iproute2
            COPY kipa_daemon /root/kipa_daemon
            COPY kipa_cli /root/kipa_cli
            WORKDIR /root
            RUN \\
                chmod +x kipa_daemon && \\
                chmod +x kipa_cli
            CMD ./kipa_daemon \\
                --key-id $KIPA_KEY_ID \\
                $KIPA_ARGS
        """)

    return docker_directory


def __create_nodes(
        client,
        key_ids: List[str],
        daemon_args: str,
        network: docker.models.networks.Network) -> Iterator[Node]:
    assert len(key_ids) < 256, "No support for more than 256 nodes"

    # Used to get a container's IP address
    api_client = docker.APIClient()

    for i, key_id in enumerate(key_ids):
        name = f"{DOCKER_PREFIX}_{i:04d}_{key_id}"

        log.info(f"Creating container with name {name}")
        container = client.containers.run(
            image=IMAGE_NAME,
            detach=True,
            name=name,
            network=network.name,
            privileged=True,  # Needed for faking poor connections
            mounts=[
                docker.types.Mount(
                    source=GPG_HOME,
                    target="/root/.gnupg",
                    type="bind",
                    read_only=False)],
            environment={"KIPA_KEY_ID": key_id, "KIPA_ARGS": daemon_args})

        ip_address = api_client.inspect_container(container.name)\
            ["NetworkSettings"]["Networks"][network.name]["IPAddress"]
        log.debug(f"Created container with IP address {ip_address}")

        yield Node(key_id, f"{ip_address}:10842", container)


def __delete_old(client) -> None:
    for container in client.containers.list(all=True):
        if not container.name.startswith(DOCKER_PREFIX):
            continue
        log.debug(f"Removing container {container.name}")
        container.remove(force=True)

    for network in client.networks.list():
        if not network.name.startswith(DOCKER_PREFIX):
            continue
        log.debug(f"Removing network {network.name}")
        network.remove()
