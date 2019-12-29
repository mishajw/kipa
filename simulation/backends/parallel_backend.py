import logging
from abc import ABC, abstractmethod
from multiprocessing.pool import ThreadPool
from typing import List, Tuple

from simulation.backends import Backend
from simulation.backends.backend import CliCommand, CliCommandResult

log = logging.getLogger(__name__)


class ParallelBackend(Backend, ABC):
    def __init__(self, num_threads: int):
        self.pool = ThreadPool(processes=num_threads)

    def run_commands(self, commands: List[CliCommand]) -> List[CliCommandResult]:
        def run(pair: Tuple[int, CliCommand]) -> CliCommandResult:
            index, command = pair
            log.info("Running command %d/%d: %s", index + 1, len(commands), command)
            result = self.run_command(command)
            log.info(
                "Finished running command %d/%d, %f seconds, success=%s",
                index + 1,
                len(commands),
                result.duration_sec,
                result.successful(),
            )
            return result

        return self.pool.map(run, enumerate(commands))

    @abstractmethod
    def run_command(self, command: CliCommand) -> CliCommandResult:
        pass
