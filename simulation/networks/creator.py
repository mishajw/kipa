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

from simulation.networks import Node
from simulation.key_creator import GPG_HOME
from simulation.networks import Network

log = logging.getLogger(__name__)

DOCKER_PREFIX = "kipa_simulation"
IMAGE_NAME = DOCKER_PREFIX
NETWORK_NAME = f"{DOCKER_PREFIX}_network"
IPV4_PREFIX = "192.168.123"
IPV6_PREFIX = "fd92:bd99:d235:d1c5::"


def create_docker_network(ipv6: bool = False):
    log.info("Creating a docker network for nodes")
    client = docker.from_env()

    if not ipv6:
        log.debug("Using IPv4")
        ipam_pool = docker.types.IPAMPool(
            subnet=f"{IPV4_PREFIX}.0/24", gateway=f"{IPV4_PREFIX}.123")
    else:
        log.debug("Using IPv6")
        ipam_pool = docker.types.IPAMPool(
            subnet=f"{IPV6_PREFIX}/64", gateway=f"{IPV6_PREFIX}123")

    return client.networks.create(
        NETWORK_NAME,
        driver="bridge",
        ipam=docker.types.IPAMConfig(
            pool_configs=[ipam_pool]),
        enable_ipv6=ipv6)


def create_containers(
        size: int,
        daemon_args: str,
        group_index: int,
        key_ids: List[str],
        network: docker.models.networks.Network,
        ipv6: bool,
        debug: bool = True) -> Network:
    """Create a network of the specified size"""

    log.info(
        f"Creating network of size {size}, "
        f"with arguments \"{daemon_args}\" and group index {group_index}")
    client = docker.from_env()

    log.info("Creating docker directory")
    docker_directory = __create_docker_directory(debug)

    log.info("Building KIPA image (may take a while)")
    client.images.build(
        path=docker_directory,
        tag=IMAGE_NAME,
        quiet=False)

    log.info("Removing docker directory")
    shutil.rmtree(docker_directory)

    log.info(f"Creating {len(key_ids)} containers")
    containers = list(__create_nodes(
        client, key_ids, group_index, daemon_args, ipv6, network))
    return Network(containers, network)


def delete_old_containers() -> None:
    log.info("Getting docker client")
    client = docker.from_env()

    log.info("Deleting old docker containers")

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


def __create_docker_directory(debug: bool) -> str:
    docker_directory = tempfile.mkdtemp()

    log.debug(f"Made docker directory at {docker_directory}")

    build_command = ["cargo", "build"]
    if not debug:
        build_command += ["--release"]

    build_process = subprocess.Popen(build_command)
    build_process.wait()
    assert build_process.returncode == 0, "KIPA build command failed"

    if debug:
        binary_directory = "target/debug"
    else:
        binary_directory = "target/release"
    daemon_binary_path = os.path.join(binary_directory, "kipa_daemon")
    cli_binary_path = os.path.join(binary_directory, "kipa_cli")

    assert os.path.isfile(daemon_binary_path)
    shutil.copyfile(
        daemon_binary_path,
        os.path.join(docker_directory, "kipa_daemon"))

    assert os.path.isfile(cli_binary_path)
    shutil.copyfile(
        cli_binary_path,
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
                chmod +x kipa_cli && \\
                echo "p@ssword" >> secret.txt
            CMD {"RUST_BACKTRACE=1" if debug else ""} ./kipa_daemon \\
                --key-id $KIPA_KEY_ID \\
                $KIPA_ARGS
        """)

    return docker_directory


def __create_nodes(
        client,
        key_ids: List[str],
        group_index: int,
        daemon_args: str,
        ipv6: bool,
        network: docker.models.networks.Network) -> Iterator[Node]:
    assert len(key_ids) < 256, "No support for more than 256 nodes"

    # Used to get a container's IP address
    api_client = docker.APIClient()

    for i, key_id in enumerate(key_ids):
        name = f"{DOCKER_PREFIX}_{group_index:02d}_{i:04d}_{key_id}"

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

        network_details = api_client.inspect_container(container.name) \
            ["NetworkSettings"]["Networks"][network.name]
        if not ipv6:
            ip_address = f"{network_details['IPAddress']}:10842"
        else:
            ip_address = f"[{network_details['GlobalIPv6Address']}]:10842"
        log.debug(f"Created container with IP address {ip_address}")

        yield Node(key_id, ip_address, container)
