import random
from pathlib import Path

from simulation.benchmarks import SuccessSpeedBenchmark
from simulation.networks import Network
from simulation.operations import simulator, TestResult

DISCONNECT_PROBABILITIES = [x / 10 for x in range(10)]


class ReliabilityBenchmark(SuccessSpeedBenchmark):
    def __init__(self, output_directory: Path):
        super().__init__(
            "reliability",
            [p * 100 for p in DISCONNECT_PROBABILITIES],
            "Disconnect probability",
            output_directory,
        )

    def get_results(self, network: Network) -> TestResult:
        for disconnect_probability in DISCONNECT_PROBABILITIES:
            disconnected_network = network.map_nodes(
                lambda n: n.replace(
                    disconnect_before_tests=random.random()
                    < disconnect_probability
                )
            )
            output_directory = (
                self.output_directory / f"prob_{disconnect_probability}"
            )
            yield simulator.simulate(disconnected_network, output_directory)
