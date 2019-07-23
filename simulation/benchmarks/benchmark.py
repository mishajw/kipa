import os
from abc import ABC, abstractmethod
from pathlib import Path
from typing import List

import matplotlib.pyplot as plt

from simulation import utils
from simulation.networks import Network
from simulation.operations import TestResult


class Benchmark(ABC):
    def __init__(self, title: str, output_directory: Path) -> None:
        output_directory = (
            output_directory / "benchmarks" / title / utils.get_formatted_time()
        )
        if not output_directory.is_dir():
            output_directory.mkdir(parents=True)

        self.title = title
        self.output_directory = output_directory

    @abstractmethod
    def create(self, network: Network) -> None:
        raise NotImplementedError()


class SuccessSpeedBenchmark(Benchmark, ABC):
    def __init__(
        self,
        title: str,
        x_values: List[float],
        x_title: str,
        output_directory: Path,
    ):
        super().__init__(title, output_directory)
        self.x_values = x_values
        self.x_title = x_title

    def create(self, network: Network):
        # Get results
        results = list(self.get_results(network))

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
            self.x_values,
            [result.success_percentage * 100 for result in results],
            "r-",
        )

        # Add speed plot
        speed_axes = success_axes.twinx()
        speed_axes.set_ylabel("Successful search time (seconds)")
        speed_axes.tick_params("y", colors="b")
        speed_axes.plot(
            self.x_values,
            [result.average_search_times_sec for result in results],
            "b-",
        )

        # Save the figure
        figure.savefig(self.output_directory / "results.png")

    @abstractmethod
    def get_results(self, network: Network) -> TestResult:
        """Get the results of simulation runs for this benchmark"""
        raise NotImplementedError()
