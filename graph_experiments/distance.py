from abc import ABC, abstractmethod

from graph_experiments import KeySpace, GraphArgs, constants


class Distance(ABC):
    def __init__(self, args: GraphArgs):
        self.args = args

    @classmethod
    def get(cls, name: str, args: GraphArgs) -> "Distance":
        if name == "wrapped":
            return Wrapped(args)
        elif name == "unwrapped":
            return Unwrapped(args)
        elif name == "ring":
            return Ring(args)
        else:
            raise AssertionError(f"Unknown distance: {name}")

    @abstractmethod
    def distance(self, a: KeySpace, b: KeySpace) -> float:
        """
        Calculates the distance between two points in key space.
        """
        raise NotImplementedError()

    @abstractmethod
    def max_distance(self) -> float:
        """
        Get the maximum distance between any two points in key space.
        """
        raise NotImplementedError()


class Wrapped(Distance):
    def distance(self, a: KeySpace, b: KeySpace) -> float:
        assert len(a.position) == len(b.position)
        total = float(0)
        for a, b in zip(a.position, b.position):
            distance = min(
                abs(a - b),
                abs((a + constants.KEY_SPACE_WIDTH) - b),
                abs((a - constants.KEY_SPACE_WIDTH) - b),
            )
            total += distance ** 2
        return total ** 0.5

    def max_distance(self) -> float:
        return (
            ((constants.KEY_SPACE_WIDTH / 2) ** 2)
            * self.args.key_space_dimensions
        ) ** 0.5


class Unwrapped(Distance):
    def distance(self, a: KeySpace, b: KeySpace) -> float:
        assert len(a.position) == len(b.position)
        total = sum((a - b) ** 2 for a, b in zip(a.position, b.position))
        return total ** 0.5

    def max_distance(self) -> float:
        return (
            ((constants.KEY_SPACE_WIDTH / 2) ** 2)
            * self.args.key_space_dimensions
        ) ** 0.5


class Ring(Distance):
    def __init__(self, args: GraphArgs):
        super().__init__(args)
        self.underlying = Wrapped(args)

    def distance(self, a: KeySpace, b: KeySpace) -> float:
        assert self.args.key_space_dimensions > 1
        radius, *a_position = a.position
        _, *b_position = b.position
        radius = abs(radius)
        return abs(
            radius
            - self.underlying.distance(
                KeySpace(a_position), KeySpace(b_position)
            )
        )

    def max_distance(self) -> float:
        return self.underlying.max_distance()
