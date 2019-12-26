import logging
import os
import re
import subprocess
import tempfile
from pathlib import Path
from typing import List, Set

log = logging.getLogger(__name__)

GPG_HOME = os.path.join(os.getcwd(), ".gnupg")
GPG_EXECUTABLE = "gpg"
GPG_ARGS = ["--homedir", GPG_HOME]


class KeyCreator:
    def __init__(self):
        self.__key_ids = self.__existing_key_ids()
        self.__used_key_ids: Set[str] = set()

    def get_key_id(self) -> str:
        remaining_keys = self.__key_ids.difference(self.__used_key_ids)
        if remaining_keys:
            key_id = next(iter(remaining_keys))
            self.__used_key_ids.add(key_id)
            log.info(f"Getting existing key {key_id}")
            return key_id

        log.info("Creating new key")

        if not os.path.isdir(GPG_HOME):
            log.debug("Creating GPG home directory")
            os.mkdir(GPG_HOME)

        key_id = self.__create_new_key()
        self.__key_ids.add(key_id)
        self.__used_key_ids.add(key_id)
        return key_id

    @staticmethod
    def __create_new_key() -> str:
        log.debug("Writing GPG commands to temp file")
        gpg_commands = tempfile.NamedTemporaryFile(mode="w")
        gpg_commands.write(
            """
            %echo Generating key for KIPA tests
            Key-Type: RSA
            Key-Length: 1024
            Subkey-Type: RSA
            Subkey-Length: 1024
            Name-Real: Test Key
            Name-Comment: Test Key
            Name-Email: test@key.com
            Expire-Date: 0
            Passphrase: p@ssword
            %commit
            %echo Finished generating key for KIPA tests
        """
        )
        gpg_commands.flush()

        log.debug("Making key...")
        gpg_output: bytes = subprocess.check_output(
            [
                GPG_EXECUTABLE,
                *GPG_ARGS,
                # No interactive
                "--batch",
                # Generate the key with the saved GPG commands
                "--generate-key",
                gpg_commands.name,
            ],
            stderr=subprocess.STDOUT,
        )
        log.debug("Finished making key")

        match = re.search(r"key ([A-F0-9]+) marked as ultimately trusted", gpg_output.decode())
        assert match, f"Failed to find key in GPG output: {gpg_output}"
        return match.group(1)[-8:]  # Get last 8 characters of fingerprint.

    @staticmethod
    def __existing_key_ids() -> Set[str]:
        log.info("Getting the number of existing keys")
        gpg_process = subprocess.Popen(
            [GPG_EXECUTABLE, *GPG_ARGS, "--list-secret-keys", "--with-colons"],
            stdout=subprocess.PIPE,
        )

        key_ids: List[str] = []
        seen_sec = False
        while True:
            line = gpg_process.stdout.readline().decode()
            if line == "":
                break
            if line.startswith("sec"):
                seen_sec = True
            if line.startswith("fpr") and seen_sec:
                seen_sec = False
                key_ids.append(KeyCreator.__key_id_from_line(line))
        gpg_process.wait()

        return set(key_ids)

    @staticmethod
    def __key_id_from_line(line: str) -> str:
        # Fingerprint is in the second to last column
        full_fingerprint = line.split(":")[-2].strip()
        # Key ID is the last eight characters for the fingerprint
        return full_fingerprint[-8:]
