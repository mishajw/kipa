import datetime


def get_formatted_time() -> str:
    return datetime.datetime.now().strftime("%Y%m%d-%H%M%S")
