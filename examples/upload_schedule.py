#!/usr/bin/env python3
"""Simple script to upload a schedule to the sat-o-mat server."""

import argparse
import re
from datetime import datetime, timedelta, timezone

import requests


def main():
    parser = argparse.ArgumentParser(description="Upload a schedule to sat-o-mat")
    parser.add_argument(
        "--schedule",
        default="schedules/basic.yml",
        help="Schedule file (default: schedules/basic.yml)",
    )
    parser.add_argument(
        "--duration",
        type=int,
        default=10,
        help="Duration in minutes (default: 10)",
    )
    parser.add_argument(
        "--server",
        default="http://localhost:8080",
        help="Server URL (default: http://localhost:8080)",
    )
    parser.add_argument(
        "--token",
        default="sk_test_admin",
        help="API token (default: sk_test_admin)",
    )
    args = parser.parse_args()

    with open(args.schedule) as f:
        schedule_yaml = f.read()

    now = datetime.now(timezone.utc)
    end = now + timedelta(minutes=args.duration)

    start_str = now.strftime("%Y-%m-%dT%H:%M:%SZ")
    end_str = end.strftime("%Y-%m-%dT%H:%M:%SZ")

    # Replace start and end variables in the YAML
    schedule_yaml = re.sub(r"start:.*", f"start: '{start_str}'", schedule_yaml)
    schedule_yaml = re.sub(r"end:.*", f"end: '{end_str}'", schedule_yaml)

    print(f"Uploading schedule: start time {start_str} -> end time {end_str}")

    response = requests.post(
        f"{args.server}/api/schedules",
        data=schedule_yaml,
        headers={
            "Content-Type": "application/yaml",
            "Authorization": f"Bearer {args.token}",
        },
    )

    if response.ok:
        print(f"Success: {response.json()}")
    else:
        print(f"Error {response.status_code}: {response.text}")


if __name__ == "__main__":
    main()
