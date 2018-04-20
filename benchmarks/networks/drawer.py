from benchmarks import networks
from benchmarks.networks import Network
from typing import List, Iterator
import operator
import re
from PIL import Image, ImageDraw


class GraphNode:
    def __init__(self, key_id: str, position: List[int], neighbours: List[str]):
        self.key_id = key_id
        self.position = position
        self.neighbours = neighbours


def draw(network: Network) -> None:
    # This will make sure that `list-neighbours` is called at the end of
    # execution
    networks.modifier.ensure_alive(network)
    graph = list(__get_nodes(network))
    __draw_graph(graph)


def __draw_graph(graph: List[GraphNode]):
    image_dims = [1920, 1080]
    node_radius = 10

    # Get the points and transpose them for bound calculation
    points = map(lambda n: n.position, graph)
    points_t = list(zip(*points))

    # Get the bounds of the dimensions
    unpadded_max_points = list(map(max, points_t))
    unpadded_min_points = list(map(min, points_t))

    # Add a padding of 10% around the bounds
    max_points = [
        _max + (_max - _min) * 0.1
        for _max, _min in zip(unpadded_max_points, unpadded_min_points)]
    min_points = [
        _min - (_max - _min) * 0.1
        for _max, _min in zip(unpadded_max_points, unpadded_min_points)]

    # Normalize the points within the bounds
    def normalize_point(ps: List[int]) -> (float, float):
        normalized = [
            ((i - _min) / (_max - _min))
            for i, _max, _min in
            zip(ps, max_points, min_points)]
        assert len(normalized) == 2, \
            f"No support for drawing >2 dimensions, found {len(normalized)}"
        return tuple(i * dim for i, dim in zip(normalized, image_dims))
    normalized_dict = dict(
        (n.key_id, normalize_point(n.position)) for n in graph)

    image = Image.new("RGBA", tuple(image_dims), color="white")
    d = ImageDraw.Draw(image)

    # Draw all neighbour connections
    for n in graph:
        (x, y) = normalized_dict[n.key_id]
        for neighbour in n.neighbours:
            (nx, ny) = normalized_dict[neighbour]
            bidirectional_neighbour = all([
                n2.key_id != neighbour or n.key_id in n2.neighbours
                for n2 in graph])

            if bidirectional_neighbour:
                # If both nodes of neighbours of each other, draw a green line
                d.line((x, y, nx, ny), fill="green", width=4)
            else:
                # If our neighbour does not have us as a neighbour, draw a half
                # green half red line, with the green half on this node's side
                (mx, my) = ((x + nx) / 2, (y + ny) / 2)
                d.line((x, y, mx, my), fill="green", width=4)
                d.line((mx, my, nx, ny), fill="red", width=4)

    # Draw all nodes as circles
    for n in graph:
        (x, y) = normalized_dict[n.key_id]
        d.ellipse(
            (
                x - node_radius,
                y - node_radius,
                x + node_radius,
                y + node_radius),
            fill="green")

    # Draw the key IDs next to the nodes
    # Done last to keep above node/neighbour drawings
    for n in graph:
        (x, y) = normalized_dict[n.key_id]
        d.text((x, y), n.key_id, fill="black")

    # Save the image
    # TODO: Add configurability to image save location
    image.save("graph.png")


def __get_nodes(network: Network) -> Iterator[GraphNode]:
    for key in network.get_all_keys():
        logs = network.get_logs(key)
        key_space = __get_key_space(logs)
        neighbours = __get_neighbours(logs)

        yield GraphNode(key, key_space, neighbours)


def __get_key_space(logs: List[dict]) -> List[int]:
    ns_logs = filter(
        lambda l: "neighbours-store" in l and l["neighbours-store"], logs)
    key_space_logs = map(operator.itemgetter("local_key_space"), ns_logs)
    groups = re.match(r"KeySpace\(([-0-9, ]+)\)", next(key_space_logs))
    return list(map(int, groups.group(1).split(", ")))


def __get_neighbours(logs: List[dict]) -> List[str]:
    flags = ["list-neighbours", "reply"]
    neighbours_logs = list(filter(
        lambda l: all(map(lambda f: f in l and l[f], flags)),
        logs))
    assert len(neighbours_logs) > 0, "Could not find logging of neighbours"
    neighbours = neighbours_logs[-1]["neighbours"]
    if neighbours == "":
        return []
    return neighbours.split(", ")
