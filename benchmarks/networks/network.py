import random
from typing import List, Dict
import json


from docker.models.containers import Container


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
        (exit_code, output) = \
            self.__key_dict[key_id].container.exec_run(command)
        output = output.decode()
        assert exit_code == 0, \
            f"Bad return code when executing command: {command}. " \
            f"Output was: {output}"
        return output

    def get_logs(self, key_id: str) -> List[dict]:
        raw_logs = self.exec_command(key_id, ["cat", "/root/log-daemon.json"])
        logs: List[dict] = []
        for line in (raw_logs.split("\n")):
            if line.strip() == "":
                continue
            logs.append(json.loads(line))
        return logs

    def get_human_readable_logs(self, key_id: str) -> str:
        logs = self.__key_dict[key_id].container.attach(
            stdout=True, stderr=True, stream=False, logs=True).decode()
        assert isinstance(logs, str), \
            f"Logs returned from docker was not a string: {logs}"
        return logs
