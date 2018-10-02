import os
from typing import Iterator

from simulation import networks
from simulation.benchmarks import SuccessSpeedBenchmark

DISCONNECT_PROBABILITIES = [x / 10 for x in range(10)]


class ReliabilityBenchmark(SuccessSpeedBenchmark):
    def __init__(self, output_directory: str):
        super().__init__(
            "reliability",
            [p * 100 for p in DISCONNECT_PROBABILITIES],
            "Disconnect probability",
            output_directory)

    def get_results(self, network_config_path: str) -> Iterator[dict]:
        for disconnect_probability in DISCONNECT_PROBABILITIES:
            configuration = networks.configuration.Configuration.from_yaml(
                network_config_path)
            configuration.disconnect_probability = disconnect_probability

            results = configuration.run(os.path.join(
                self.output_directory,
                f"prob_{disconnect_probability}"))
            yield results
