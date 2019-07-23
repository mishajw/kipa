from abc import ABC, abstractmethod
from multiprocessing.pool import ThreadPool
from typing import List

from simulation.backends import Backend
from simulation.backends.backend import CliCommand, CliCommandResult


class ParallelBackend(Backend, ABC):
    def __init__(self, num_threads: int):
        self.pool = ThreadPool(processes=num_threads)

    def run_commands(
        self, commands: List[CliCommand]
    ) -> List[CliCommandResult]:
        return self.pool.map(self.run_command, commands)

    @abstractmethod
    def run_command(self, command: CliCommand) -> CliCommandResult:
        pass
