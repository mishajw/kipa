import logging
import os
import subprocess
import tempfile
from typing import List

log = logging.getLogger(__name__)

GPG_HOME = os.path.join(os.getcwd(), ".gnupg")
GPG_EXECUTABLE = "gpg"
GPG_ARGS = ["--homedir", GPG_HOME]

def create_keys(num: int) -> List[str]:
    if not os.path.isdir(GPG_HOME):
        os.mkdir(GPG_HOME)

    existing_key_ids = __get_existing_key_ids()
    num_keys_to_create = num - len(existing_key_ids)

    if num_keys_to_create <= 0:
        log.info(
            f"Found {len(existing_key_ids)}, "
            f"asked for {num} keys, "
            f"not creating any more keys")
    else:
        log.info(
            f"Found {len(existing_key_ids)}, "
            f"asked for {num} keys, "
            f"creating {num_keys_to_create}")

    log.debug("Writing GPG commands to temp file")
    gpg_commands = tempfile.NamedTemporaryFile(mode="w")
    gpg_commands.write("""
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
    """)
    gpg_commands.flush()

    for _ in range(num_keys_to_create):
        log.debug("Making key...")
        gpg_process = subprocess.Popen(
            [
                GPG_EXECUTABLE, *GPG_ARGS,
                # No interactive
                "--batch",
                # Generate the key with the saved GPG commands
                "--generate-key", gpg_commands.name],
            stdout=subprocess.PIPE)
        gpg_process.wait()
        assert gpg_process.returncode == 0, \
            f"Key creation failed with code {gpg_process.returncode}"
        log.debug("Finished making key")

    key_ids = __get_existing_key_ids()
    assert len(key_ids) >= num, "Too few keys created"
    log.info(f"Total of {len(key_ids)} in keyring")
    return key_ids[:num]

def __get_existing_key_ids() -> List[str]:
    log.info("Getting the number of existing keys")
    gpg_process = subprocess.Popen(
        [
            GPG_EXECUTABLE, *GPG_ARGS,
            "--list-secret-keys", "--with-colons"],
        stdout=subprocess.PIPE)

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
            # Fingerprint is in the second to last column
            full_fingerprint = line.split(":")[-2].strip()
            # Key ID is the last eight characters for the fingerprint
            key_id = full_fingerprint[-8:]
            key_ids.append(key_id)
    gpg_process.wait()

    return key_ids
