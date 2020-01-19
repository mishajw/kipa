import random
from pathlib import Path

from simulation.benchmarks import SuccessSpeedBenchmark
from simulation.networks import Network

DISCONNECT_PROBABILITIES = [x / 10 for x in range(10)]


class ReliabilityBenchmark(SuccessSpeedBenchmark):
    def __init__(self, output_directory: Path):
        super().__init__(
            "reliability", DISCONNECT_PROBABILITIES, "Disconnect probability", output_directory,
        )

    def get_network(self, network: Network, disconnect_probability: float) -> Network:
        return network.map_nodes(
            lambda n: n.replace(disconnect_before_tests=random.random() < disconnect_probability)
        )

    def format_parameter(self, parameter: float) -> str:
        return str(parameter * 100)
