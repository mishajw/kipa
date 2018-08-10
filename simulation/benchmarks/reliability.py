import os
from typing import Iterator

from simulation import networks, utils
import matplotlib.pyplot as plt

DISCONNECT_PROBABILITIES = [x / 10 for x in range(11)]


def run_reliability_benchmark(network_config_path: str, output_directory: str):
    output_directory = os.path.join(
        output_directory, "benchmarks/reliability", utils.get_formatted_time())
    if not os.path.isdir(output_directory):
        os.makedirs(output_directory)

    results = list(__get_results(network_config_path, output_directory))

    plt.title("Reliability benchmark")
    plt.xlabel("Disconnect probability (%)")
    plt.ylabel("Search success (%)")
    plt.plot(
        [p * 100 for p in DISCONNECT_PROBABILITIES],
        [r * 100 for r in results])
    plt.savefig(os.path.join(output_directory, "results.png"))


def __get_results(
        network_config_path: str, output_directory: str) -> Iterator[float]:
    for disconnect_probability in DISCONNECT_PROBABILITIES:
        configuration = networks.configuration.Configuration.from_yaml(
            network_config_path)
        configuration.disconnect_probability = disconnect_probability

        results = configuration.run(os.path.join(
            output_directory,
            f"prob_{disconnect_probability}"))
        yield results["percentage_success"]
