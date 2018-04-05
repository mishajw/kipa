"""
Code for creating networks of docker containers.
"""

from benchmarks.key_creator import create_keys, GPG_HOME
from docker.models.containers import Container
from docker.models.networks import Network
from typing import Iterator, List
import docker
import logging
import os
import random
import shutil
import subprocess
import tempfile

log = logging.getLogger(__name__)

DOCKER_PREFIX = "kipa_benchmark"
IMAGE_NAME = DOCKER_PREFIX
NETWORK_NAME = f"{DOCKER_PREFIX}_network"
IPV4_PREFIX = "192.168.123"

class Network:
    def __init__(self, key_ids: List[str], containers: list) -> None:
        self.__containers = containers
        self.__key_ids = key_ids
        self.__key_dict = dict(zip(self.__key_ids, self.__containers))

    def test_search(self, from_key_id: str, to_key_id: str) -> bool:
        log.info(f"Testing search between {from_key_id} and {to_key_id}")
        (exit_code, output) = self.__key_dict[from_key_id].exec_run(
            [
                "/root/kipa_cli",
                "--socket-path", "/tmp/kipa",
                "search",
                "--key-id", to_key_id])

        log.info(
                f"Returned exit code {exit_code}, "
                f"and output:\n{output.decode()}")

        return False

    def get_random_keys(self, num: int) -> List[str]:
        return random.sample(self.__key_ids, num)

    def get_all_keys(self) -> List[str]:
        return self.__key_ids


def create_network(size: int) -> Network:
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
    containers = list(__create_containers(client, key_ids, network))

    return Network(key_ids, containers)


def __create_docker_directory() -> str:
    docker_directory = tempfile.mkdtemp()

    log.debug(f"Made docker directory at {docker_directory}")

    build_process = subprocess.Popen(["cargo", "build", "--release"])
    build_process.wait()
    assert build_process.returncode == 0, "Build command failed"

    assert os.path.isfile("target/release/kipa_daemon")
    shutil.copyfile(
        "target/release/kipa_daemon",
        os.path.join(docker_directory, "kipa_daemon"))

    assert os.path.isfile("target/release/kipa_cli")
    shutil.copyfile(
        "target/release/kipa_cli",
        os.path.join(docker_directory, "kipa_cli"))

    with open(os.path.join(docker_directory, "Dockerfile"), "w") as f:
        f.write(f"""
            FROM debian:stretch-slim
            ENV KIPA_KEY_ID ""
            ENV KIPA_ARGS ""
            RUN \\
                apt-get update && apt-get --yes install gpg
            COPY kipa_daemon /root/kipa_daemon
            COPY kipa_cli /root/kipa_cli
            WORKDIR /root
            RUN \\
                chmod +x kipa_daemon && \\
                chmod +x kipa_cli
            CMD ./kipa_daemon \\
                --socket-path /tmp/kipa \\
                --key-id $KIPA_KEY_ID \\
                $KIPA_ARGS
        """)

    return docker_directory


def __create_containers(
        client, key_ids: List[str], network: Network) -> Iterator[Container]:

    for i, key_id in enumerate(key_ids):
        name = f"{DOCKER_PREFIX}_{i}_{key_id}"
        ip_address = f"{IPV4_PREFIX}.{i + 1}"

        # Every kipa node is connected to the next
        next_ip_address = f"{IPV4_PREFIX}.{((i + 1) % len(key_ids)) + 1}"

        log.info(f"Creating container with name {name}")
        container = client.containers.run(
            image=IMAGE_NAME,
            detach=True,
            name=name,
            mounts=[
                docker.types.Mount(
                    source=GPG_HOME,
                    target="/root/.gnupg",
                    type="bind",
                    read_only=True)],
            environment={
                "KIPA_KEY_ID": key_id,
                "KIPA_ARGS":
                    "--port 12345 "
                    f"--initial-node-address {next_ip_address}:12345 "
                    f"--initial-node-key-id {key_id}"})

        log.debug(
            f"Adding container {name} "
            f"to network {network.name} "
            f"with IP address {ip_address}")
        network.connect(container, ipv4_address=ip_address)

        yield container


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

