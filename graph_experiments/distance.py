from abc import ABC, abstractmethod

from graph_experiments import KeySpace, Args

# TODO: DRY
KEY_SPACE_LOWER = -1
KEY_SPACE_UPPER = 1
KEY_SPACE_WIDTH = KEY_SPACE_UPPER - KEY_SPACE_LOWER


class Distance(ABC):
    @classmethod
    def get(cls, name: str) -> "Distance":
        if name == "wrapped":
            return Wrapped()
        elif name == "unwrapped":
            return Unwrapped()
        else:
            raise AssertionError(f"Unknown distance: {name}")

    @abstractmethod
    def distance(self, a: KeySpace, b: KeySpace) -> float:
        """
        Calculates the distance between two points in key space.
        """
        raise NotImplementedError()

    @abstractmethod
    def max_distance(self, args: Args) -> float:
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
                abs((a + KEY_SPACE_WIDTH) - b),
                abs((a - KEY_SPACE_WIDTH) - b),
            )
            total += distance ** 2
        return total ** 0.5

    def max_distance(self, args: "Args") -> float:
        return (((KEY_SPACE_WIDTH / 2) ** 2) * args.key_space_dimensions) ** 0.5


class Unwrapped(Distance):
    def distance(self, a: KeySpace, b: KeySpace) -> float:
        assert len(a.position) == len(b.position)
        total = sum((a - b) ** 2 for a, b in zip(a.position, b.position))
        return total ** 0.5

    def max_distance(self, args: "Args") -> float:
        return (((KEY_SPACE_WIDTH / 2) ** 2) * args.key_space_dimensions) ** 0.5
