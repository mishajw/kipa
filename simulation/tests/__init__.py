import logging

from simulation.tests import test_search

logging.basicConfig()
logging.getLogger().setLevel(logging.DEBUG)
logging.getLogger("docker").setLevel(logging.WARNING)
logging.getLogger("urllib3").setLevel(logging.WARNING)
