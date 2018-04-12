import random
from typing import List


class Network:
    def __init__(self, key_ids: List[str], containers: list) -> None:
        self.__containers = containers
        self.__key_ids = key_ids
        self.__key_dict = dict(zip(self.__key_ids, self.__containers))

    def get_random_keys(self, num: int) -> List[str]:
        return random.sample(self.__key_ids, num)

    def get_all_keys(self) -> List[str]:
        return self.__key_ids

    def exec_command(self, key_id: str, command: List[str]) -> str:
        (exit_code, output) = self.__key_dict[key_id].exec_run(command)
        output = output.decode()
        assert exit_code == 0, \
            f"Bad return code when executing command: {command}. " \
            f"Output was: {output}"
        return output
