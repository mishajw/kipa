import random
from typing import List, Dict
import json
import logging
import docker
from docker.models.containers import Container

log = logging.getLogger(__name__)


class Node:
    def __init__(self, key_id: str, address: str, container: Container):
        self.key_id = key_id
        self.address = address
        self.container = container


class Network:
    def __init__(self, nodes: List[Node]) -> None:
        self.__nodes = nodes
        self.__key_ids = [n.key_id for n in self.__nodes]
        self.__key_dict: Dict[str, Node] = dict(
            [(n.key_id, n) for n in self.__nodes])

    def get_random_keys(self, num: int) -> List[str]:
        return random.sample(self.__key_ids, num)

    def get_all_keys(self) -> List[str]:
        return self.__key_ids

    def get_address(self, key_id: str) -> str:
        return self.__key_dict[key_id].address

    def exec_command(self, key_id: str, command: List[str]) -> str:
        try:
            (exit_code, output) = \
                self.__key_dict[key_id].container.exec_run(command)
        except docker.errors.APIError as e:
            container_logs = self.__key_dict[key_id].container.logs().decode()
            log.error(
                f"Error on {key_id} when performing command {command}, "
                f"logs: {container_logs}")
            raise e
        output = output.decode()
        assert exit_code == 0, \
            f"Bad return code when executing command: {command}. " \
            f"Output was: {output}"
        return output

    def get_logs(self, key_id: str) -> List[dict]:
        return self.__get_logs_from_file(key_id, "/root/log-daemon.json")

    def get_cli_logs(self, key_id: str) -> List[dict]:
        return self.__get_logs_from_file(key_id, "/root/log-cli.json")

    def __get_logs_from_file(self, key_id: str, file_name: str) -> List[Dict]:
        raw_logs = self.exec_command(key_id, ["cat", file_name])
        logs: List[dict] = []
        for line in (raw_logs.split("\n")):
            if line.strip() == "":
                continue
            try:
                json_dict = json.loads(line)
            except json.decoder.JSONDecodeError as e:
                log.warning(f"Failed to decode JSON string: {line}, error: {e}")
                continue
            logs.append(json_dict)

        return logs

    def get_human_readable_logs(self, key_id: str) -> bytes:
        logs = self.__key_dict[key_id].container.attach(
            stdout=True, stderr=True, stream=False, logs=True)
        assert isinstance(logs, bytes), \
            f"Logs returned from docker was not bytes: {logs}"
        return logs
