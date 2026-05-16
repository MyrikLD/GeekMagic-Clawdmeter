#!/usr/bin/env python3
import argparse
import json
from http.client import HTTPResponse
from pathlib import Path
from urllib.request import Request, urlopen


def get_token(credentials: Path) -> str:
    creds = json.load(credentials.expanduser().open())
    return creds["claudeAiOauth"]["accessToken"]


def send(host: str, token: str) -> int:
    req = Request(f"http://{host}/token", data=token.encode(), method="POST")
    with urlopen(req) as resp:
        resp: HTTPResponse
        return not (resp.status == 200 and resp.read().decode().strip() == "OK")


def main():
    parser = argparse.ArgumentParser(description="Push Anthropic token to clawdmeter device")
    parser.add_argument(
        "host",
        help="Device IP or hostname",
    )
    parser.add_argument(
        "-c",
        "--credentials",
        default="~/.claude/.credentials.json",
        metavar="PATH",
        help="Path to credentials JSON (default: ~/.claude/.credentials.json)",
        type=Path,
    )
    args = parser.parse_args()

    token = get_token(args.credentials)
    return send(args.host, token)


if __name__ == "__main__":
    exit(main())
