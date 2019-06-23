import logging
import os
from typing import Iterator

from simulation import networks
from simulation.benchmarks import SuccessSpeedBenchmark

log = logging.getLogger(__name__)

NETWORK_SIZES = list(range(20, 201, 20))


class ScalabilityBenchmark(SuccessSpeedBenchmark):
    def __init__(self, output_directory: str):
        super().__init__(
            "scalability", NETWORK_SIZES, "Network size", output_directory
        )

    def get_results(self, network_config_path: str) -> Iterator[dict]:
        for network_size in NETWORK_SIZES:
            configuration = networks.configuration.Configuration.from_yaml(
                network_config_path
            )
            # Adjust network size of every group in the network configuration so
            # that the sum of all groups is `network_size`
            configuration_network_size = sum(
                group.size for group in configuration.groups
            )
            network_size_multiplier = network_size / configuration_network_size
            for group in configuration.groups:
                group.size = int(group.size * network_size_multiplier)
            log.debug(
                f"Running with group sizes: "
                f"{[group.size for group in configuration.groups]}"
            )

            results = configuration.run(
                os.path.join(self.output_directory, f"size_{network_size}")
            )
            yield results
