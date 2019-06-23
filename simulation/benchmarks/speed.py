import logging
import os
from typing import Iterator

from simulation import networks
from simulation.benchmarks import SuccessSpeedBenchmark

log = logging.getLogger(__name__)

CONNECTION_QUALITIES = [x / 100 for x in range(0, 100, 10)]
"""
Speed qualities ranging from 0% loss, 0ms delay, and 1mbps, to 90% loss, 1
second delay, and 10kbps
"""


class SpeedBenchmark(SuccessSpeedBenchmark):
    def __init__(self, output_directory: str):

        super().__init__(
            "speed",
            CONNECTION_QUALITIES,
            "Speed quality (0-1 scale)",
            output_directory,
        )

    def get_results(self, network_config_path: str) -> Iterator[dict]:
        for quality_rating in CONNECTION_QUALITIES:
            quality = networks.configuration.ConnectionQuality(
                loss=quality_rating,
                delay=quality_rating * 1000,
                rate=(1 - quality_rating) * 1000,
            )

            configuration = networks.configuration.Configuration.from_yaml(
                network_config_path
            )

            for group in configuration.groups:
                group.connection_quality = quality
                # Set timeout high so that the daemon does not exit searches that
                # take too long
                group.daemon_args["search_timeout_sec"] = 10000

            results = configuration.run(
                os.path.join(
                    self.output_directory,
                    f"conn_qual_{quality.loss}_{quality.delay}_{quality.rate}",
                )
            )
            yield results["average_search_time_sec"]
