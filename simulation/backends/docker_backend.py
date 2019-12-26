import json
import logging
import shutil
import tempfile
import time
from pathlib import Path
from typing import List, Dict, Tuple, Optional

import docker
from docker.models.containers import Container

from simulation import Build
from simulation.backends import CliCommand, CliCommandResult
from simulation.backends import ParallelBackend
from simulation.key_creator import GPG_HOME
from simulation.networks import Network, Node, NodeId, ConnectionQuality

log = logging.getLogger(__name__)

DOCKER_PREFIX = "kipa_simulation"
IMAGE_PREFIX = f"{DOCKER_PREFIX}_image"
NETWORK_NAME = f"{DOCKER_PREFIX}_network"
IPV4_PREFIX = "172.16"
IPV6_PREFIX = "fd92:bd99:d235:d1c5::"


class DockerBackend(ParallelBackend):
    def __init__(self, num_threads: int):
        super().__init__(num_threads)
        self.__containers: Dict[NodeId, Container] = {}
        self.__ip_addresses: Dict[NodeId, str] = {}

        self.__client = docker.from_env()
        self.__api_client = docker.APIClient()
        self.__network: Optional[Network] = None

    def initialize_network(self, network: Network, node_builds: Dict[NodeId, Build]) -> None:
        self.__network = self.__create_network(network)

        log.info("Creating docker images")
        builds = set(node_builds.values())
        build_to_image = {build: self.__create_docker_image(build) for build in builds}

        log.info(f"Creating {len(network.nodes)} containers")
        self.__containers = {}
        self.__ip_addresses = {}
        for node in network.nodes:
            image = build_to_image[node_builds[node.id]]
            container, ip_address = self.__create_container(node, image, network)
            self.__containers[node.id] = container
            self.__ip_addresses[node.id] = ip_address
            # FIXME: If we don't sleep, we run out of memory when GPG reads keys, causing daemon
            # startups to fail.
            time.sleep(1)

        self.__fake_poor_connection(network.connection_quality)

    def get_ip_address(self, node_id: NodeId) -> str:
        return self.__ip_addresses[node_id]

    def run_command(self, command: CliCommand) -> CliCommandResult:
        start_sec = time.time()
        output = self.__run_container_command(command.node_id, ["/root/kipa_cli", *command.args])
        duration_sec = time.time() - start_sec
        if output is None:
            return CliCommandResult.failed(command)

        res = CliCommandResult(command, output, self.get_cli_logs(command.node_id), duration_sec)
        return res

    def stop_networking(self, node_id: NodeId):
        self.__network.disconnect(self.__containers[node_id])

    def get_logs(self, node_id: NodeId) -> List[dict]:
        return self.__get_logs_from_file(node_id, "/root/logs/log-daemon.json")

    def get_cli_logs(self, node_id: NodeId) -> List[dict]:
        return self.__get_logs_from_file(node_id, "/root/logs/log-cli.json")

    def get_human_readable_logs(self, node_id: NodeId) -> bytes:
        logs = self.__containers[node_id].attach(stdout=True, stderr=True, stream=False, logs=True)
        assert isinstance(logs, bytes), f"Logs returned from docker was not bytes: {logs}"
        return logs

    def clean(self) -> None:
        log.info("Deleting old docker containers")

        for container in self.__client.containers.list(all=True):
            if not container.name.startswith(DOCKER_PREFIX):
                continue
            log.debug(f"Removing container {container.name}")
            container.remove(force=True)

        for network in self.__client.networks.list():
            if not network.name.startswith(DOCKER_PREFIX):
                continue
            log.debug(f"Removing network {network.name}")
            network.remove()

    def __create_network(self, network: Network):
        if not network.ipv6:
            log.debug("Using IPv4")
            ipam_pool = docker.types.IPAMPool(
                subnet=f"{IPV4_PREFIX}.0.0/16", gateway=f"{IPV4_PREFIX}.0.123"
            )
        else:
            log.debug("Using IPv6")
            ipam_pool = docker.types.IPAMPool(
                subnet=f"{IPV6_PREFIX}/64", gateway=f"{IPV6_PREFIX}123"
            )

        return self.__client.networks.create(
            NETWORK_NAME,
            driver="bridge",
            ipam=docker.types.IPAMConfig(pool_configs=[ipam_pool]),
            enable_ipv6=network.ipv6,
        )

    def __create_docker_image(self, build: Build) -> str:
        docker_directory = Path(tempfile.mkdtemp(suffix=build.id()))
        log.debug(f"Made docker directory at {docker_directory}")

        # TODO: Docker requires COPY files to be in the docker directory,
        # meaning we copy the builds twice.
        shutil.copy(str(build.cli_path), str(docker_directory / "kipa_cli"))
        shutil.copy(str(build.daemon_path), str(docker_directory / "kipa_daemon"))

        log.debug("Creating Dockerfile")
        with open(docker_directory / "Dockerfile", "w") as f:
            # TODO: Base docker image has to use the same `glibc` as host
            # machine
            f.write(
                f"""
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
                CMD RUST_BACKTRACE=1 ./kipa_daemon \\
                    -vvvv \\
                    --key-id $KIPA_KEY_ID \\
                    $KIPA_ARGS
            """
            )

        image_name = f"{IMAGE_PREFIX}_{build.id()}"
        log.info(f"Building KIPA image {image_name} (may take a while)")
        self.__client.images.build(path=str(docker_directory), tag=image_name, quiet=False)

        log.info(f"Removing docker directory at {docker_directory}")
        shutil.rmtree(docker_directory)

        return image_name

    def __create_container(
        self, node: Node, image_name: str, network: Network
    ) -> Tuple[Container, str]:
        container_name = f"{DOCKER_PREFIX}_{node.id}"

        log.info(f"Creating container with name {container_name}")
        container = self.__client.containers.run(
            image=image_name,
            detach=True,
            name=container_name,
            network=self.__network.name,
            privileged=True,  # Needed for faking poor connections
            mounts=[
                docker.types.Mount(
                    source=GPG_HOME, target="/root/.gnupg", type="bind", read_only=False,
                )
            ],
            environment={
                "KIPA_KEY_ID": node.key_id(),
                "KIPA_ARGS": " ".join(
                    f"--{k.replace('_', '-')} {v}"
                    for k, v in node.daemon_args.items()
                ),
            },
        )

        network_details = self.__api_client.inspect_container(container.name)["NetworkSettings"][
            "Networks"
        ][self.__network.name]
        if not network.ipv6:
            ip_address = f"{network_details['IPAddress']}:10842"
        else:
            ip_address = f"[{network_details['GlobalIPv6Address']}]:10842"
        log.debug(f"Created container with IP address {ip_address}")

        return container, ip_address

    def __run_container_command(self, node_id: NodeId, command: List[str]) -> Optional[str]:
        try:
            (exit_code, output) = self.__containers[node_id].exec_run(command)
        except docker.errors.APIError as error:
            container_logs = self.__containers[node_id].logs().decode()
            log.error(
                f"Error on {node_id} when performing command {command}, "
                f"logs: {container_logs}. Returning empty string. "
                f"Error: {error}"
            )
            return None

        output = output.decode()
        if exit_code != 0:
            log.error(
                f"Bad return code when executing command: {command}. " f"Output was: {output}"
            )
            # TODO: Correct behaviour?
            return None

        return output

    def __get_logs_from_file(self, node_id: NodeId, file_name: str) -> List[Dict]:
        raw_logs = self.__run_container_command(node_id, ["cat", file_name])
        logs: List[dict] = []
        for line in raw_logs.split("\n"):
            if line.strip() == "":
                continue
            try:
                json_dict = json.loads(line)
            except json.decoder.JSONDecodeError as e:
                log.warning(f"Failed to decode JSON string: {line}, error: {e}")
                continue
            logs.append(json_dict)
        return logs

    def __fake_poor_connection(self, quality: Optional[ConnectionQuality]) -> None:
        if quality is None:
            return

        log.debug(
            "Faking a poor connection between all containers with "
            f"loss {quality.loss * 100:.3f}%, "
            f"delay {quality.delay}, and "
            f"rate of {quality.rate}Kbps"
        )

        command = (
            f"tc qdisc add dev eth0 root netem "
            + (f"loss {quality.loss * 100}% " if quality.loss != 0 else "")
            + (f"delay {quality.delay} " if quality.delay != 0 else "")
            + (f"rate {quality.rate}kbit" if quality.rate != 0 else "")
        )

        for container in self.__containers.values():
            container.exec_run(command.split(" "))
