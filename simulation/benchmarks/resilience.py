import logging
import random
from pathlib import Path
from typing import Iterator

from simulation.benchmarks import SuccessSpeedBenchmark
from simulation.networks import Network
from simulation.operations import simulator

log = logging.getLogger(__name__)

MALICIOUS_PROBABILITIES = [x / 10 for x in range(10)]


class ResilienceBenchmark(SuccessSpeedBenchmark):
    def __init__(self, output_directory: Path):
        super().__init__(
            "resilience",
            [p * 100 for p in MALICIOUS_PROBABILITIES],
            "Malicious probability (%)",
            output_directory,
        )

    def get_results(self, network: Network) -> Iterator[dict]:
        for malicious_probability in MALICIOUS_PROBABILITIES:
            malicious_network = network.map_nodes(
                lambda n: n.replace(
                    additional_features=random.random() < malicious_probability
                )
            )
            output_directory = (
                self.output_directory / f"prob_{malicious_probability}"
            )
            yield simulator.simulate(malicious_network, output_directory)
