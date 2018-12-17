import itertools
import json
import logging
import os
import random
from enum import Enum
from typing import Dict, Any, List

import yaml

from simulation import networks, key_creator

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


class ConnectionQuality:
    def __init__(self, loss: float, delay: float, rate: float) -> None:
        self.loss = loss
        self.delay = delay
        self.rate = rate

    @classmethod
    def from_dict(cls, d: dict) -> "ConnectionQuality":
        return cls(
            d["loss"] if "loss" in d else 0,
            d["delay"] if "delay" in d else 0,
            d["rate"] if "rate" in d else 0)


class GroupConfiguration:
    """Configuration of a group of nodes in a network"""

    def __init__(
            self,
            size: int,
            daemon_args: Dict[str, str] = None,
            connection_quality: ConnectionQuality = None,
            ipv6: bool = False,
            additional_features: List[str] = None,
            clear_default_features: bool = False,
            test_searches: bool = True):
        self.size = size
        self.daemon_args = daemon_args if daemon_args is not None else {}
        self.connection_quality = connection_quality
        self.ipv6 = ipv6
        self.additional_features = additional_features
        self.clear_default_features = clear_default_features
        self.test_searches = test_searches

    @classmethod
    def from_dict(cls, d: dict) -> "GroupConfiguration":
        assert "size" in d, "Missing size field in group configuration"
        return cls(
            d["size"],
            d["daemon_args"] if "daemon_args" in d else {},
            ConnectionQuality.from_dict(d["connection_quality"])
            if "connection_quality" in d else None,
            d["ipv6"]
            if "ipv6" in d else False,
            d["additional_features"] if "additional_features" in d else [],
            d["clear_default_features"]
            if "clear_default_features" in d else False,
            d["test_searches"] if "test_searches" in d else True)

    def get_daemon_args_str(self) -> str:
        args = self.daemon_args
        # Ensure that "False" is given as "false"
        for key in args:
            if type(args[key]) != bool:
                continue
            args[key] = str(args[key]).lower()
        args_str = " ".join(
            [f"--{arg.replace('_', '-')} {args[arg]}"
             for arg in args])

        if self.ipv6:
            args_str += " --force-ipv6=true"

        return args_str


class Configuration:
    def __init__(
            self,
            groups: List[GroupConfiguration],
            connect_type: ConnectType,
            num_connects: int,
            num_search_tests: int = None,
            debug: bool = True,
            original_parameters: dict = None,
            disconnect_probability: float = 0.0,
            keep_alive: bool = False):
        if original_parameters is None:
            original_parameters = {}
        self.groups = groups
        self.connect_type = connect_type
        self.num_connects = num_connects
        self.num_search_tests = num_search_tests
        self.debug = debug
        self.original_parameters = original_parameters
        self.keep_alive = keep_alive

        self.disconnect_probability = disconnect_probability
        """
        Probability of a node not being reachable during a search, resampled
        every search test
        """

    @classmethod
    def from_yaml(cls, yaml_path: str) -> "Configuration":
        with open(yaml_path, "r") as f:
            parameters = yaml.load(f)
        assert "groups" in parameters and type(parameters["groups"]) == list, \
            "Missing groups list in configuration"

        return cls(
            [GroupConfiguration.from_dict(group)
             for group in parameters["groups"]],
            ConnectType.from_str(parameters["connect_type"]),
            parameters["num_connects"],
            parameters["num_search_tests"]
            if "num_search_tests" in parameters else None,
            parameters["debug"]
            if "debug" in parameters else True,
            keep_alive=parameters["keep_alive"]
            if "keep_alive" in parameters else False,
            original_parameters=parameters)

    def run(self, output_directory: str) -> dict:
        """
        Run the configuration, print to `stdout` the result as well as returning
        it.
        """

        # The results dictionary that will be written detailing the
        # configuration run
        results_dict: Dict[str, Any] = dict(
            original_config=self.original_parameters)

        # Create the directory for outputting configuration run data
        if not os.path.isdir(output_directory):
            os.makedirs(output_directory)

        networks.creator.delete_old_containers()

        log.info("Creating keys")
        key_ids = iter(
            key_creator.create_keys(sum(g.size for g in self.groups)))

        log.info(f"Creating network with {len(self.groups)} groups")
        docker_network = networks.creator.create_docker_network(
            ipv6=any(g.ipv6 for g in self.groups))
        network = networks.Network([], docker_network)
        for i, group in enumerate(self.groups):
            log.info(f"Creating group with {group.size} nodes")
            group_network = networks.creator.create_containers(
                group.size,
                group.get_daemon_args_str(),
                i,
                list(itertools.islice(key_ids, group.size)),
                docker_network,
                group.test_searches,
                group.additional_features,
                group.clear_default_features,
                group.ipv6,
                self.debug)
            if group.connection_quality is not None:
                log.info("Setting connection quality")
                networks.modifier.fake_poor_connection(
                    group_network,
                    group.connection_quality.loss,
                    group.connection_quality.delay,
                    group.connection_quality.rate)
            network += group_network
        results_dict["keys"] = network.get_all_keys()

        # Create the `connect` function for connecting all nodes
        if self.connect_type == ConnectType.ROOTED:
            [root_key_id] = network.get_random_keys(1)

            def connect():
                networks.modifier.connect_nodes_to_one(network, root_key_id)
        elif self.connect_type == ConnectType.CYCLICAL:
            def connect():
                networks.modifier.connect_nodes_cyclically(network)
        else:
            raise ValueError(f"Unhandled `ConnectType`: {self.connect_type}")

        log.info("Ensuring all nodes in network are alive")
        networks.modifier.ensure_alive(network)
        for i in range(self.num_connects):
            log.info(f"Performing connection #{i + 1}")
            connect()

        if self.disconnect_probability != 0:
            log.info(
                "Disconnecting nodes with probability "
                f"{self.disconnect_probability * 100:.2f}%")
            for key_id in network.get_all_keys():
                if random.random() < self.disconnect_probability:
                    log.debug(f"Disconnecting {key_id}")
                    network.stop_networking(key_id)

        log.info("Getting search results")
        search_results = networks.tester.sample_test_searches(
            network, num_searches=self.num_search_tests)
        percentage_success = search_results.percentage_success()
        average_num_requests = search_results.average_num_requests()
        average_search_time_sec = search_results.average_search_time_sec()
        results_dict["percentage_success"] = percentage_success
        results_dict["average_num_requests"] = average_num_requests
        results_dict["average_search_time_sec"] = average_search_time_sec
        log.info(f"Search results: {percentage_success * 100}% success, "
                 f"average {average_num_requests} requests, "
                 f"average {average_search_time_sec} seconds per search")

        log.info("Getting logs")
        # This will call `list-neighbours` so that we have an up-to-date account
        # of each node's neighbours in the logs
        networks.modifier.ensure_alive(network)
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
            with open(os.path.join(network_log_dir, f"{key}.txt"), "wb") as f:
                f.write(network_human_readable_logs[key])

        log.info("Drawing main graph")
        graph_directory = os.path.abspath(
            os.path.join(output_directory, "graphs"))
        if not os.path.isdir(graph_directory):
            os.makedirs(graph_directory)
        main_graph_path = os.path.join(graph_directory, "graph.png")
        networks.drawer.draw_main_graph(
            network_logs, main_graph_path)
        results_dict["graph"] = "file://" + main_graph_path

        log.info("Drawing search networks and collecting search results")
        results_dict["search_results"] = []
        for i in range(len(search_results)):
            from_key_id, to_key_id, result, message_id, num_requests, \
            search_time_sec = \
                search_results[i]
            query_graph_path = os.path.join(
                graph_directory, f"{message_id}.png")
            networks.drawer.draw_query_graph(
                network_logs,
                from_key_id,
                to_key_id,
                message_id,
                query_graph_path)
            results_dict["search_results"].append(dict(
                from_key_id=from_key_id,
                to_key_id=to_key_id,
                success=result,
                message_id=message_id,
                num_requests=num_requests,
                search_time_sec=search_time_sec,
                graph="file://" + query_graph_path))

        with open(os.path.join(output_directory, "details.yaml"), "w") as f:
            yaml.dump(results_dict, f, default_flow_style=False)

        if not self.keep_alive:
            networks.creator.delete_old_containers()

        return results_dict
