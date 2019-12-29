import datetime
import itertools
import json
import sys
from typing import List, Iterator, Tuple

from dateutil.parser import parse as parse_time


def main(log_path: str):
    # Read the logs
    with open(log_path) as f:
        logs: List[dict] = json.load(f)

    # Get the time between each log
    elapsed_times_sec = list(__get_elapsed_times_sec(logs))

    # Get the slowest 100 logs individually
    slowest_individual = itertools.islice(
        sorted(elapsed_times_sec, key=lambda t: t[1], reverse=True), 100
    )

    slowest_grouped = sorted(
        [
            (message, sum(t for _, t in logs))
            for message, logs in itertools.groupby(
                sorted(elapsed_times_sec, key=lambda t: t[0]["msg"]), key=lambda t: t[0]["msg"]
            )
        ],
        key=lambda t: t[1],
        reverse=True,
    )

    print("Slowest individual:")
    for i, (log, elapsed_time_sec) in enumerate(slowest_individual):
        print(f"#{i + 1}: Took {elapsed_time_sec} seconds, " f"message: {log['msg']}")
        print(f"Full log: {log}")

    print("Slowest grouped:")
    for i, (message, elapsed_time_sec) in enumerate(slowest_grouped):
        print(f"#{i + 1}: Took {elapsed_time_sec} seconds, " f"message: {message}")


def __get_elapsed_times_sec(logs: Iterator[dict]) -> Iterator[Tuple[dict, int]]:
    for prev_log, cur_log in zip(logs, logs[1:]):
        prev_time = __get_time(prev_log)
        cur_time = __get_time(cur_log)
        time_diff_sec = (cur_time - prev_time).microseconds / 1e6
        yield cur_log, time_diff_sec


def __get_time(log: dict) -> datetime.datetime:
    time_str = log["ts"]
    return parse_time(time_str)


if __name__ == "__main__":
    assert len(sys.argv) == 2, f"Usage: {sys.argv[0]} <log path>"
    main(sys.argv[1])
