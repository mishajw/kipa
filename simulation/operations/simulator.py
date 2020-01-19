import logging
from pathlib import Path
from typing import Dict, Tuple, Any

import yaml

from simulation.backends import DockerBackend
from simulation.networks import Network
from simulation.operations import (
    create_builds,
    draw_main_graph,
    draw_query_graph,
    NetworkLogs,
    TestResult,
    sample_test_searches,
    connect_network,
)
from simulation.operations import get_logs, write_logs

log = logging.getLogger(__name__)


def simulate(network: Network, output_directory: Path) -> TestResult:
    log.info("Starting backend")
    backend = DockerBackend(network.num_threads)
    backend.clean()

    log.info("Building and initializing network")
    node_builds = create_builds(network.nodes)
    backend.initialize_network(network, node_builds)

    log.info("Connecting network together")
    connect_network(network, backend)

    log.info("Disconnecting some nodes")
    for node in network.nodes:
        if node.disconnect_before_tests:
            backend.stop_networking(node.id)

    log.info("Running tests on network")
    test_results = sample_test_searches(network, backend, network.num_searches)

    log.info("Getting logs and cleaning up backend")
    logs = get_logs(network, backend)
    backend.clean()

    log.info("Creating search graphs")
    main_graph_path, query_graph_paths = __write_graphs(test_results, logs, output_directory)

    log.info("Writing out test results")
    report = __build_report(network, test_results, main_graph_path, query_graph_paths)
    with open(str(output_directory / "report.yaml"), "w") as file:
        yaml.dump(report, file, default_flow_style=False)
    write_logs(logs, output_directory)

    log.info(
        "Results: %.2f%% successful, %.2fs avg, %.2f avg requests",
        test_results.success_percentage * 100,
        test_results.average_search_times_sec,
        test_results.average_num_requests,
    )
    return test_results


def __write_graphs(
    results: TestResult, logs: NetworkLogs, output_directory: Path
) -> Tuple[Path, Dict[str, Path]]:
    log.info("Drawing all graphs")

    graph_directory = (output_directory / "graphs").absolute()
    if not graph_directory.is_dir():
        graph_directory.mkdir(parents=True)

    main_graph_path = graph_directory / "graph.png"
    draw_main_graph(logs, main_graph_path)

    query_graph_paths: Dict[str, Path] = {}
    for result in results.search_results:
        query_graph_path = graph_directory / f"{result.message_id}.png"
        draw_query_graph(
            logs, result.from_id, result.to_id, result.message_id, query_graph_path,
        )
        query_graph_paths[result.message_id] = query_graph_path
    return main_graph_path, query_graph_paths


def __build_report(
    network: Network,
    results: TestResult,
    main_graph_path: Path,
    query_graph_paths: Dict[str, Path],
) -> Dict[str, Any]:
    report: Dict[str, Any] = {
        "key_ids": [node_id.key_id for node_id in network.ids()],
        "graph": str(main_graph_path),
        "success_percentage": results.success_percentage,
        "average_num_requests": results.average_num_requests,
        "average_search_times_sec": results.average_search_times_sec,
        "searches": [],
    }

    for search_result in results.search_results:
        report["searches"].append(
            {
                "from_id": search_result.from_id.key_id,
                "to_id": search_result.to_id.key_id,
                "success": search_result.success,
                "message_id": search_result.message_id,
                "num_requests": search_result.num_requests,
                "search_times_sec": search_result.search_times_sec,
                "graph": str(query_graph_paths[search_result.message_id]),
            }
        )

    return report
