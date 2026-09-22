import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import re
import selectors
import signal
import subprocess
import sys
import time
import uuid

ROOT = Path(__file__).resolve().parents[1]
MAX_LOG_BYTES = 8 * 1024 * 1024


class IncompleteOutput(ValueError):
    pass


class IncorrectOutput(ValueError):
    pass


def read_pi_fixture(path):
    digits = Path(path).read_bytes().strip()
    if len(digits) != 1001 or not digits.isdigit() or not digits.startswith(b"314159265358979323846264338327950288419716939937510"):
        raise ValueError("Pi fixture must contain the provided C program's 1,001 decimal digits")
    return digits


def check_pi(data, start, expected):
    match = re.search(rb"(?m)^pi ~= ([0-9])\.", data[start:])
    if match is None:
        raise IncompleteOutput("waiting for 'pi ~= ' and the decimal point")
    pos = start + match.end()
    digits = bytearray(match.group(1))
    while pos < len(data) and data[pos] in b"0123456789\r\n":
        if 48 <= data[pos] <= 57:
            digits.append(data[pos])
        pos += 1
    if len(digits) < len(expected):
        if pos < len(data):
            raise IncorrectOutput(f"pi ended after {len(digits)} digits; expected {len(expected)}")
        raise IncompleteOutput(f"waiting for all pi digits ({len(digits)}/{len(expected)})")
    if bytes(digits) != expected:
        raise IncorrectOutput("pi digits differ from the supplied C program's frozen output")
    if pos == len(data) and not data.endswith(b"\n"):
        raise IncompleteOutput("waiting for the pi line terminator")
    return pos


def check_output(data, expected_pi, mode="batch"):
    if mode == "pi":
        check_pi(data, 0, expected_pi)
        return {"mode": "pi", "pi_digits": len(expected_pi)}
    if mode != "batch":
        raise ValueError(f"unknown output mode: {mode}")
    hellos = list(re.finditer(rb"(?m)^Hello world from [^\r\n]+!\r?\n", data))
    if not hellos:
        raise IncompleteOutput("waiting for the kernel greeting")
    if len(hellos) != 1:
        raise IncorrectOutput("expected one kernel greeting; found repeated boots or concurrent output")
    first = re.search(rb"(?m)^1[ \t]+1[ \t]+1\r?\n", data[hellos[0].end():])
    if first is None:
        raise IncompleteOutput("waiting for squares row '1 1 1' after the greeting")
    pos = hellos[0].end() + first.start()
    for number in range(1, 101):
        end = data.find(b"\n", pos)
        if end < 0:
            raise IncompleteOutput(f"waiting for squares row {number}")
        fields = data[pos:end].split()
        wanted = [str(value).encode("ascii") for value in (number, number * number, 2 * number - 1)]
        if fields != wanted:
            raise IncorrectOutput(f"squares row {number}: expected {wanted!r}, got {fields!r}")
        pos = end + 1
    pos = check_pi(data, pos, expected_pi)
    milestone = re.search(rb"(?m)^primecheck: Found another 1000 primes; last one was ([0-9]+)!\r?\n", data[pos:])
    if milestone is None:
        raise IncompleteOutput("waiting for the first primecheck milestone after user pi")
    if int(milestone.group(1)) != 7919:
        raise IncorrectOutput("the first 1,000-prime milestone must be 7919")
    return {"mode": "batch", "squares_rows": 100, "pi_digits": len(expected_pi), "first_1000th_prime": 7919}


def artifact_hashes():
    result = {}
    for name in ("kernel8.img", "armstub.bin", "user/squares.elf", "user/pi.elf", "user/primecheck.elf"):
        path = ROOT / name
        if path.is_file():
            result[name] = hashlib.sha256(path.read_bytes()).hexdigest()
    return result


def run_qemu(args, expected):
    if os.name != "posix":
        raise ValueError("automated QEMU capture requires a POSIX host; use --log on other hosts")
    directory = ROOT / "target" / "test-results"
    directory.mkdir(parents=True, exist_ok=True)
    stem = directory / f"qemu-{args.mode}-{uuid.uuid4().hex}"
    log_path = stem.with_suffix(".log")
    command = ["make", "--no-print-directory", "qemu"]
    data = bytearray()
    report = {"source": "local QEMU subprocess", "command": command, "mode": args.mode, "timeout_seconds": args.timeout, "passed": False}
    process = None
    try:
        process = subprocess.Popen(command, cwd=ROOT, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, start_new_session=True)
        deadline = time.monotonic() + args.timeout
        pending = "waiting for output"
        with log_path.open("xb") as log, selectors.DefaultSelector() as selector:
            selector.register(process.stdout, selectors.EVENT_READ)
            while time.monotonic() < deadline:
                events = selector.select(min(0.25, max(0, deadline - time.monotonic())))
                if not events:
                    continue
                chunk = os.read(process.stdout.fileno(), 65536)
                if not chunk:
                    raise IncorrectOutput(f"build/QEMU output closed before completion (status {process.poll()}): {pending}")
                log.write(chunk)
                log.flush()
                data.extend(chunk)
                if len(data) > MAX_LOG_BYTES:
                    raise IncorrectOutput("capture exceeded 8 MiB without satisfying the output checks")
                try:
                    report.update(check_output(bytes(data), expected, args.mode))
                except IncompleteOutput as error:
                    pending = str(error)
                    continue
                report["passed"] = True
                return report
            raise IncompleteOutput(f"timeout after {args.timeout:g}s: {pending}")
    except (ValueError, OSError, subprocess.SubprocessError) as error:
        report["error"] = str(error)
        raise
    finally:
        if process is not None:
            try:
                os.killpg(process.pid, signal.SIGTERM)
            except ProcessLookupError:
                pass
            try:
                process.wait(timeout=3)
            except subprocess.TimeoutExpired:
                os.killpg(process.pid, signal.SIGKILL)
                process.wait()
            if process.stdout is not None:
                process.stdout.close()
        report["artifact_sha256"] = artifact_hashes()
        report["log"] = str(log_path.relative_to(ROOT))
        stem.with_suffix(".json").write_text(json.dumps(report, indent=2) + "\n")
        print(f"Capture and report: {stem.relative_to(ROOT)}.{{log,json}}", file=sys.stderr)


def main():
    parser = argparse.ArgumentParser(description="Local Project 1 output checks, not the official grader or a UART register test.")
    source = parser.add_mutually_exclusive_group(required=True)
    source.add_argument("--log", type=Path, help="read an existing raw serial log; does not authenticate its origin")
    source.add_argument("--qemu", action="store_true", help="run and stop only a new, owned make qemu process group")
    parser.add_argument("--mode", choices=("pi", "batch"), default="batch")
    parser.add_argument("--timeout", type=float, default=120, help="QEMU/build deadline in seconds; adjust for a slow host")
    parser.add_argument("--pi-fixture", type=Path, default=ROOT / "tests" / "pi_bcd.txt")
    args = parser.parse_args()
    try:
        if not math.isfinite(args.timeout) or args.timeout <= 0:
            raise ValueError("timeout must be a finite positive number")
        expected = read_pi_fixture(args.pi_fixture)
        if args.qemu:
            report = run_qemu(args, expected)
        else:
            if args.log.stat().st_size > MAX_LOG_BYTES:
                raise ValueError("log exceeds the 8 MiB capture limit; provide one boot's output")
            data = args.log.read_bytes()
            if len(data) > MAX_LOG_BYTES:
                raise ValueError("log exceeds the 8 MiB capture limit; provide one boot's output")
            report = check_output(data, expected, args.mode)
            report.update(passed=True, source="unverified supplied log", log_sha256=hashlib.sha256(data).hexdigest())
        print(json.dumps(report, indent=2))
        return 0
    except (ValueError, OSError, subprocess.SubprocessError) as error:
        print(f"FAIL: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
