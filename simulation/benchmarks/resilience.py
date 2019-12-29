import logging
import random
from pathlib import Path

from simulation.benchmarks import SuccessSpeedBenchmark
from simulation.networks import Network

log = logging.getLogger(__name__)

MALICIOUS_PROBABILITIES = [x / 10 for x in range(10)]


class ResilienceBenchmark(SuccessSpeedBenchmark):
    def __init__(self, output_directory: Path):
        super().__init__(
            "resilience", MALICIOUS_PROBABILITIES, "Malicious probability (%)", output_directory,
        )

    def get_network(self, network: Network, malicious_probability: float) -> Network:
        return network.map_nodes(
            lambda n: n.replace(additional_features=random.random() < malicious_probability)
        )

    def format_parameter(self, parameter: float) -> str:
        return str(parameter * 100)
