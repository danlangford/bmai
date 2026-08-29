# SPDX-License-Identifier: MIT
# SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

"""Minimal persistent JSONL client using only Python's standard library."""

from __future__ import annotations

import json
import subprocess
from pathlib import Path
from typing import Any


class Bmair:
    def __init__(self, executable: str | Path = "bmair") -> None:
        self._process = subprocess.Popen(
            [str(executable), "--protocol", "jsonl-v1"],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=None,
            text=True,
            bufsize=1,
        )
        self._next_id = 1

    def request(self, method: str, params: dict[str, Any] | None = None) -> Any:
        request_id = self._next_id
        self._next_id += 1
        request = {"protocol": "jsonl-v1", "id": request_id, "method": method}
        if params is not None:
            request["params"] = params
        assert self._process.stdin is not None
        assert self._process.stdout is not None
        self._process.stdin.write(json.dumps(request, separators=(",", ":")) + "\n")
        self._process.stdin.flush()
        line = self._process.stdout.readline()
        if not line:
            raise RuntimeError(f"BMAIR exited with status {self._process.poll()}")
        response = json.loads(line)
        if response.get("protocol") != "jsonl-v1":
            raise RuntimeError(f"protocol mismatch: {response!r}")
        if response.get("id") != request_id:
            raise RuntimeError(f"response ID mismatch: {response!r}")
        if not response.get("ok"):
            raise RuntimeError(response["error"])
        return response["result"]

    def close(self) -> None:
        if self._process.stdin is not None:
            self._process.stdin.close()
        status = self._process.wait()
        if status:
            raise RuntimeError(f"BMAIR exited with status {status}")

    def __enter__(self) -> Bmair:
        return self

    def __exit__(self, *_: object) -> None:
        self.close()


def main() -> None:
    with Bmair() as bmair:
        print(json.dumps(bmair.request("capabilities"), indent=2))
        result = bmair.request("session.execute", {"script": "seed 17\nply 1\n"})
        print(json.dumps(result["session"], indent=2))


if __name__ == "__main__":
    main()
