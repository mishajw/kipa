from pathlib import Path
from typing import NamedTuple


class Build(NamedTuple):
    cli_path: Path
    daemon_path: Path

    def id(self) -> str:
        return hex(hash(self))[-8:]
