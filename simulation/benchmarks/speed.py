import logging
import os
from typing import Iterator

import matplotlib.pyplot as plt

from simulation import networks, utils

log = logging.getLogger(__name__)

CONNECTION_QUALITIES = [x / 100 for x in range(0, 100, 10)]
"""
Speed qualities ranging from 0% loss, 0ms delay, and 1mbps, to 90% loss, 1
second delay, and 10kbps
"""


def run_speed_benchmark(network_config_path: str, output_directory: str):
    output_directory = os.path.join(
        output_directory, "benchmarks/speed", utils.get_formatted_time())
    if not os.path.isdir(output_directory):
        os.makedirs(output_directory)

    results = list(__get_results(network_config_path, output_directory))

    plt.title("Speed benchmark")
    plt.xlabel("Speed quality (0-1 scale)")
    plt.ylabel("Average search time (seconds)")
    plt.plot(
        CONNECTION_QUALITIES,
        results)
    plt.savefig(os.path.join(output_directory, "results.png"))


def __get_results(
        network_config_path: str, output_directory: str) -> Iterator[float]:
    for quality_rating in CONNECTION_QUALITIES:
        quality = networks.configuration.ConnectionQuality(
            loss=quality_rating,
            delay=quality_rating * 1000,
            rate=(1 - quality_rating) * 1000)

        configuration = networks.configuration.Configuration.from_yaml(
            network_config_path)

        for group in configuration.groups:
            group.connection_quality = quality
            # Set timeout high so that the daemon does not exit searches that
            # take too long
            group.daemon_args["search_timeout_sec"] = 10000

        results = configuration.run(os.path.join(
            output_directory,
            f"conn_qual_{quality.loss}_{quality.delay}_{quality.rate}"))
        yield results["average_search_time_sec"]
