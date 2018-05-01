import json
import logging
import os
from enum import Enum
import datetime

import yaml

from benchmarks import networks

log = logging.getLogger(__name__)


class ConnectType(Enum):
    CYCLICAL = 0
    ROOTED = 1

    @classmethod
    def from_str(cls, s: str) -> "ConnectType":
        if s == "cyclical":
            return ConnectType.CYCLICAL
        elif s == "rooted":
            return ConnectType.ROOTED
        else:
            raise ValueError(f"Unrecognized `ConnectType`: {s}")

    def to_str(self) -> str:
        if self == ConnectType.CYCLICAL:
            return "cyclical"
        elif self == ConnectType.ROOTED:
            return "rooted"
        else:
            raise ValueError(f"Unhandled `ConnectType`: {self}")


class Configuration:
    def __init__(
            self, num_nodes: int, connect_type: ConnectType, num_connects: int):
        self.num_nodes = num_nodes
        self.connect_type = connect_type
        self.num_connects = num_connects

    @classmethod
    def from_yaml(cls, yaml_path: str) -> "Configuration":
        with open(yaml_path, "r") as f:
            parameters = yaml.load(f)
        return cls(
            parameters["num_nodes"],
            ConnectType.from_str(parameters["connect_type"]),
            parameters["num_connects"])

    def run(self, output_directory: str) -> dict:
        """
        Run the configuration, print to `stdout` the result as well as returning
        it.
        """

        # The results dictionary that will be written detailing the
        # configuration run
        results_dict = dict(
            original_config=dict(
                num_nodes=self.num_nodes,
                connect_type=self.connect_type.to_str(),
                num_connects=self.num_connects))

        # Create the directory for outputting configuration run data
        time_str = datetime.datetime.now().strftime("%Y%m%d-%H%M%S")
        output_directory = os.path.join(
            output_directory, f"configuration_{time_str}")
        if not os.path.isdir(output_directory):
            os.makedirs(output_directory)

        log.info(f"Creating network of size {self.num_nodes}")
        network = networks.creator.create(self.num_nodes)
        results_dict["keys"] = network.get_all_keys()

        # Create the `connect` function for connecting all nodes
        if self.connect_type == ConnectType.CYCLICAL:
            [root_key_id] = network.get_random_keys(1)

            def connect():
                networks.modifier.connect_nodes_to_one(network, root_key_id)
        elif self.connect_type == ConnectType.ROOTED:
            def connect():
                networks.modifier.connect_nodes_cyclically(network)
        else:
            raise ValueError(f"Unhandled `ConnectType`: {self.connect_type}")

        log.info("Ensuring all nodes in network are alive")
        networks.modifier.ensure_alive(network)
        for i in range(self.num_connects):
            log.info(f"Performing connection #{i + 1}")
            connect()

        log.info("Getting logs")
        network_logs = dict()
        network_human_readable_logs = dict()
        for key in network.get_all_keys():
            network_logs[key] = network.get_logs(key)
            network_human_readable_logs[key] = network.get_human_readable_logs(
                key)

        log.info("Saving logs")
        network_log_dir = os.path.join(output_directory, "logs")
        if not os.path.isdir(network_log_dir):
            os.makedirs(network_log_dir)
        for key in network_logs.keys():
            with open(os.path.join(network_log_dir, f"{key}.json"), "w") as f:
                json.dump(network_logs[key], f)
            with open(os.path.join(network_log_dir, f"{key}.txt"), "w") as f:
                f.write(network_human_readable_logs[key])

        log.info("Drawing network")
        graph_path = os.path.abspath(
            os.path.join(output_directory, "graph.png"))
        networks.drawer.draw(
            network_logs, graph_path)
        results_dict["graph"] = graph_path

        log.info("Getting search results")
        search_results = networks.tester.sample_test_searches(network)
        percentage_success = search_results.percentage_success()
        results_dict["percentage_success"] = percentage_success
        log.info(f"Search results: {percentage_success * 100}% success")

        with open(os.path.join(output_directory, "details.yaml"), "w") as f:
            yaml.dump(results_dict, f, default_flow_style=False)

        return results_dict
