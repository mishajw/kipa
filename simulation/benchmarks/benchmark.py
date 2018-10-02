import os
from typing import List, Iterator

import matplotlib.pyplot as plt

from simulation import utils


class Benchmark:
    def __init__(self, title: str, output_directory: str) -> None:
        output_directory = os.path.join(
            output_directory, f"benchmarks/{title}", utils.get_formatted_time())
        if not os.path.isdir(output_directory):
            os.makedirs(output_directory)

        self.title = title
        self.output_directory = output_directory

    def create(self, network_config_path: str) -> None:
        raise NotImplementedError()


class SuccessSpeedBenchmark(Benchmark):
    def __init__(
            self,
            title: str,
            x_values: List[float],
            x_title: str,
            output_directory: str):
        super().__init__(title, output_directory)
        self.x_values = x_values
        self.x_title = x_title

    def create(self, network_config_path: str):
        # Get results
        results = list(self.get_results(network_config_path))

        # Create matplotlib figure
        figure = plt.figure()
        success_axes = figure.add_subplot(111)

        # Set title and x label
        success_axes.set_title(self.title.capitalize())
        success_axes.set_xlabel(self.x_title)

        # Add search success plot
        success_axes.set_ylabel("Search success (%)")
        success_axes.tick_params("y", colors="r")
        success_axes.plot(
            self.x_values, [r["percentage_success"] * 100 for r in results], "r-")

        # Add speed plot
        speed_axes = success_axes.twinx()
        speed_axes.set_ylabel("Successful search time (seconds)")
        speed_axes.tick_params("y", colors="b")
        speed_axes.plot(
            self.x_values, [r["average_search_time_sec"] for r in results], "b-")

        # Save the figure
        figure.savefig(os.path.join(self.output_directory, "results.png"))

    def get_results(self, network_config_path: str) -> Iterator[dict]:
        """Get the results of simulation runs for this benchmark"""
        raise NotImplementedError()
