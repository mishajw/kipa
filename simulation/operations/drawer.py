import logging
import operator
import re
from pathlib import Path
from typing import List, Iterator, Dict, Tuple, NamedTuple

from PIL import Image, ImageDraw

from simulation.networks import NodeId
from simulation.operations import NetworkLogs, NodeLogs

log = logging.getLogger(__name__)

IMAGE_DIMS = [1920, 1080]
NODE_RADIUS = 10


class GraphNode(NamedTuple):
    node_id: NodeId
    position: List[int]


def draw_main_graph(logs: NetworkLogs, save_path: Path) -> None:
    key_to_node = {node.key_id: node for node in logs.node_ids()}
    graph = list(__get_nodes(logs))
    neighbours = list(__get_neighbours(logs, key_to_node))
    neighbours = __remove_fake_neighbours(graph, neighbours)
    location_dict = __get_location_dict(graph, IMAGE_DIMS)
    image = Image.new("RGBA", tuple(IMAGE_DIMS), color="white")
    draw = ImageDraw.Draw(image)
    __draw_neighbours(neighbours, location_dict, draw)
    __draw_nodes(graph, location_dict, draw)
    image.save(save_path)


def draw_query_graph(
    logs: NetworkLogs, from_id: NodeId, to_id: NodeId, message_id: str, save_path: Path,
) -> None:
    key_to_node = {node.key_id: node for node in logs.node_ids()}
    graph = list(__get_nodes(logs))
    message_neighbours = list(__get_message_neighbours(logs.get(from_id), message_id, key_to_node))
    message_neighbours = __remove_fake_neighbours(graph, message_neighbours)
    location_dict = __get_location_dict(graph, IMAGE_DIMS)
    image = Image.new("RGBA", tuple(IMAGE_DIMS), color="white")
    draw = ImageDraw.Draw(image)
    __draw_neighbours(message_neighbours, location_dict, draw)
    __draw_nodes(graph, location_dict, draw)

    if from_id in location_dict:
        __draw_node_circle(location_dict[from_id], draw, color="blue")
    else:
        log.warning("from_key_id not in location_dict")
    if to_id in location_dict:
        __draw_node_circle(location_dict[to_id], draw, color="red")
    else:
        log.warning("to_key_id not in location_dict")

    try:
        image.save(save_path)
    except ValueError as e:
        # TODO: Fix issue with PIL throwing "unknown file extension" errors
        log.warning(f"Failed to write image with error: {e}")


def __draw_nodes(
    graph: List[GraphNode], location_dict: Dict[NodeId, Tuple[float, float]], draw: ImageDraw,
):
    """Draw all nodes as circles"""

    for n in graph:
        __draw_node_circle(location_dict[n.node_id], draw)

    # Draw the key IDs next to the nodes
    # Done last to keep above node/neighbour drawings
    for n in graph:
        (x, y) = location_dict[n.node_id]
        draw.text((x, y), n.node_id.key_id, fill="black")


def __draw_node_circle(centre: Tuple[float, float], draw: ImageDraw, color: str = "green"):
    x, y = centre
    draw.ellipse(
        (x - NODE_RADIUS, y - NODE_RADIUS, x + NODE_RADIUS, y + NODE_RADIUS), fill=color,
    )


def __draw_neighbours(
    neighbours: List[Tuple[NodeId, NodeId]],
    location_dict: Dict[NodeId, Tuple[float, float]],
    draw: ImageDraw,
):
    """Draw all neighbour connections"""

    for from_node, to_node in neighbours:
        (ax, ay) = location_dict[from_node]
        (bx, by) = location_dict[to_node]
        bidirectional_neighbour = (to_node, from_node) in neighbours

        if bidirectional_neighbour:
            # If both nodes of neighbours of each other, draw a green line
            draw.line((ax, ay, bx, by), fill="green", width=4)
        else:
            # If our neighbour does not have us as a neighbour, draw a half
            # green half red line, with the green half on this node's side
            (mx, my) = ((ax + bx) / 2, (ay + by) / 2)
            draw.line((ax, ay, mx, my), fill="green", width=4)
            draw.line((mx, my, bx, by), fill="red", width=4)


def __get_nodes(logs: NetworkLogs) -> Iterator[GraphNode]:
    for node_id in logs.node_ids():
        ns_logs = filter(
            lambda l: "neighbours_store" in l and l["neighbours_store"] and "local_key_space" in l,
            logs.get(node_id).logs,
        )
        key_space_logs = list(map(operator.itemgetter("local_key_space"), ns_logs))
        if len(key_space_logs) == 0:
            continue
        groups = re.match(r"KeySpace\(([-0-9, ]+)\)", key_space_logs[0])
        key_space = list(map(int, groups.group(1).split(", ")))

        yield GraphNode(node_id, key_space)


def __get_neighbours(
    logs: NetworkLogs, key_to_node: Dict[str, NodeId]
) -> Iterator[Tuple[NodeId, NodeId]]:
    for node_id in logs.node_ids():
        flags = ["list_neighbours", "reply"]
        neighbours_logs = list(
            filter(lambda l: all(map(lambda f: f in l and l[f], flags)), logs.get(node_id).logs,)
        )
        if len(neighbours_logs) == 0:
            return iter([])
        neighbours = neighbours_logs[-1]["neighbour_keys"]

        if neighbours == "":
            continue

        for n in neighbours.split(", "):
            yield (node_id, key_to_node[n])


def __get_message_neighbours(
    node_logs: NodeLogs, message_id: str, key_to_node: Dict[str, NodeId]
) -> Iterator[Tuple[NodeId, NodeId]]:
    message_logs = [
        l for l in node_logs.logs if "message_id" in l and l["message_id"] == message_id
    ]

    for l in message_logs:
        if "found" not in l:
            continue
        key_id = __get_key_id_from_string(l["node"])
        neighbours = l["neighbours"].split(", ") if l["neighbours"] != "" else []
        yield from [(key_to_node[key_id], key_to_node[n]) for n in neighbours]


def __get_location_dict(
    graph: List[GraphNode], image_dims: List[int]
) -> Dict[NodeId, Tuple[float, float]]:
    # Get the points and transpose them for bound calculation
    points = map(lambda n: n.position, graph)
    points_t = list(zip(*points))

    # Get the bounds of the dimensions
    unpadded_max_points = list(map(max, points_t))
    unpadded_min_points = list(map(min, points_t))

    # Add a padding of 10% around the bounds
    max_points = [
        _max + (_max - _min) * 0.1 for _max, _min in zip(unpadded_max_points, unpadded_min_points)
    ]
    min_points = [
        _min - (_max - _min) * 0.1 for _max, _min in zip(unpadded_max_points, unpadded_min_points)
    ]

    # Normalize the points within the bounds
    def normalize_point(ps: List[int]) -> Tuple[float, float]:
        normalized = [
            ((i - _min) / (_max - _min)) for i, _max, _min in zip(ps, max_points, min_points)
        ]
        if len(normalized) == 1:
            normalized.append(0)
        if len(normalized) != 2:
            log.warning(f"No support for drawing !=2 dimensions, " f"found {len(normalized)}")
            normalized = normalized[:2]
        return tuple(float(i * dim) for i, dim in zip(normalized, image_dims))[:2]

    return dict((n.node_id, normalize_point(n.position)) for n in graph)


def __get_key_id_from_string(s: str) -> str:
    groups = list(re.finditer(r"Key\(([0-9A-F]+)\)", s))
    assert len(groups) == 1, f"Could not find exactly one key in {s}"
    key_id = groups[0].group(1)
    assert len(key_id) == 8, f"Found key that was not 8 long: {key_id}"
    return key_id


def __remove_fake_neighbours(
    graph: List[GraphNode], neighbours: List[Tuple[NodeId, NodeId]]
) -> List[Tuple[NodeId, NodeId]]:
    neighbour_ids = set(n for ns in neighbours for n in ns)
    graph_ids = [node.node_id for node in graph]

    if not neighbour_ids.issubset(graph_ids):
        log.error(
            f"Found neighbours that were not in the graph: "
            f"found {neighbour_ids}, "
            f"graph was {[node.node_id.key_id for node in graph]}"
        )

    return [(a, b) for a, b in neighbours if a in graph_ids and b in graph_ids]
