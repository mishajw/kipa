import itertools
import json
import logging
import operator
from collections import Counter
from pathlib import Path
from typing import Dict, List


NOTIFY_EVENTS = [
    "SearchError",
    "SearchNotFound",
    "SearchVerificationFailed",
    "QueryFailed",
]


def main():
    latest_cli_run = max(Path("simulation_output/cli").iterdir())
    print(f"Looking at run {latest_cli_run}")

    json_logs = list((latest_cli_run / "logs").glob("*.json"))
    json_logs = {l: get_events(l) for l in json_logs}
    for json_log, events in json_logs.items():
        print(f"JSON log {json_log.name}")
        print_stats(events)
        print_by_message_id(events)
        print("")

    all_events = [e for events in json_logs.values() for e in events]
    print(f"All logs")
    print_stats(all_events)


def get_events(json_log: Path) -> List[Dict]:
    events = [e for e in json.loads(json_log.read_text()) if "log_event" in e]
    assert all("message_id" in e for e in events), "All events must have a message_id"
    events = [{**json.loads(e["log_event"]), "message_id": e["message_id"]} for e in events]
    return events


def print_stats(events: List[Dict]) -> None:
    event_types = (e["type"] for e in events)
    print("Events:")
    for event_type, count in Counter(event_types).items():
        print("-", event_type, count, sep="\t")

    request_types = (
        next(iter(e["payload"].keys())) for e in events if e["type"] == "ReceiveRequest"
    )
    print("Requests:")
    for request_type, count in Counter(request_types).items():
        print("-", request_type, count, sep="\t")

    message_ids = {e["message_id"] for e in events}
    print(f"Message IDs: {len(message_ids)}")


def print_by_message_id(events: List[Dict]) -> None:
    key = operator.itemgetter("message_id")
    for message_id, events in itertools.groupby(sorted(events, key=key), key):
        events = list(events)
        if not any(e["type"] in NOTIFY_EVENTS for e in events):
            continue

        print(f"Message ID: {message_id}")
        for event in events:
            print("-", json.dumps(event), sep="\t")


if __name__ == "__main__":
    logging.basicConfig()
    logging.getLogger().setLevel(logging.DEBUG)
    main()
