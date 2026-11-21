"""Paired, release-only benchmark of the same SevenZip API against two revisions.

Requires Python 3.12+, two isolated directories containing release-built
DriverIndexer.exe/archive_backend.exe, and a 7z.exe oracle with its matching DLL.
All generated output stays under --output. No drivers are installed.
"""
import argparse
import csv
import hashlib
import json
import os
import platform
import shutil
import statistics
import stat
import subprocess
import time
from pathlib import Path


def run(command):
    started = time.perf_counter_ns()
    result = subprocess.run([str(value) for value in command], capture_output=True)
    milliseconds = (time.perf_counter_ns() - started) / 1_000_000
    if result.returncode:
        raise RuntimeError(f"Command failed ({result.returncode}): {command}\n"
                           + result.stdout.decode("utf-8", errors="replace") + "\n"
                           + result.stderr.decode("utf-8", errors="replace"))
    return result, milliseconds


def digest(path):
    checksum = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            checksum.update(chunk)
    return checksum.hexdigest()


def manifest(root):
    return {path.relative_to(root).as_posix(): (path.stat().st_size, digest(path))
            for path in sorted(root.rglob("*")) if path.is_file()}


def percentile(values, quantile):
    values = sorted(values)
    position = (len(values) - 1) * quantile
    left = int(position)
    return values[left] + (values[min(left + 1, len(values) - 1)] - values[left]) * (position - left)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--main-bin", required=True, type=Path)
    parser.add_argument("--bit7z-bin", required=True, type=Path)
    parser.add_argument("--oracle", required=True, type=Path)
    parser.add_argument("--archive", action="append", required=True, help="NAME=PATH")
    parser.add_argument("--output", required=True, type=Path, help="A new, empty directory")
    parser.add_argument("--repeats", type=int, default=10)
    parser.add_argument("--compression-repeats", type=int, default=6)
    parser.add_argument("--warmups", type=int, default=2)
    parser.add_argument("--operations", default="metadata,inf_cat,directory,full,compress,index")
    arguments = parser.parse_args()
    if arguments.repeats < 2 or arguments.compression_repeats < 2 or arguments.warmups < 1:
        parser.error("At least two repetitions and one warmup are required")
    operations = arguments.operations.split(",")
    if set(operations) - {"metadata", "inf_cat", "directory", "full", "compress", "index"}:
        parser.error("Unknown operation")
    root = arguments.output.resolve()
    if root.exists() and any(root.iterdir()):
        parser.error("Output directory must be empty")
    root.mkdir(parents=True, exist_ok=True)
    oracle = arguments.oracle.resolve()
    binaries = {"main": arguments.main_bin.resolve(), "bit7z": arguments.bit7z_bin.resolve()}
    for directory in binaries.values():
        for name in ["DriverIndexer.exe", "archive_backend.exe"]:
            if not (directory / name).is_file():
                parser.error(f"Missing binary: {directory / name}")
        for name in ["7z.exe", "7z.dll"]:
            if (directory / name).exists():
                parser.error(f"Embedded-resource benchmark must not have adjacent {name}: {directory}")

    def remove_owned(path):
        path = path.resolve()
        if path == root or not path.is_relative_to(root):
            raise RuntimeError(f"Refusing to remove path outside benchmark output: {path}")
        if path.is_dir():
            def clear_readonly(function, filename, exception):
                if not isinstance(exception, PermissionError):
                    raise exception
                os.chmod(filename, stat.S_IWRITE)
                function(filename)
            shutil.rmtree(path, onexc=clear_readonly)
        elif path.exists():
            path.unlink()

    metadata = {"platform": platform.platform(), "python": platform.python_version(),
                "repeats": arguments.repeats, "compression_repeats": arguments.compression_repeats,
                "warmups": arguments.warmups, "cache_policy": "warm filesystem cache; no cache flushing",
                "order": "alternating main/bit7z within each paired round, one process at a time",
                "resource_policy": "embedded runtime resources; none adjacent to executables",
                "binaries": {version: {name: {"bytes": (directory / name).stat().st_size,
                                                "sha256": digest(directory / name)}
                                       for name in ["DriverIndexer.exe", "archive_backend.exe"]}
                             for version, directory in binaries.items()}, "packages": {}}
    for specification in arguments.archive:
        name, path = specification.split("=", 1)
        if not name or Path(name).name != name or name in {".", ".."} or name in metadata["packages"]:
            parser.error("Package names must be unique directory names")
        archive = Path(path).resolve(strict=True)
        source = root / "reference" / name
        run([oracle, "x", archive, f"-o{source}", "-y", "-p"])
        expected = manifest(source)
        infs = sorted(path for path in expected if path.lower().endswith(".inf"))
        if not infs:
            raise RuntimeError(f"No INF in {archive}")
        folder = infs[0].rsplit("/", 1)[0] if "/" in infs[0] else ""
        metadata["packages"][name] = {"archive": str(archive), "archive_bytes": archive.stat().st_size,
            "archive_sha256": digest(archive), "files": len(expected),
            "uncompressed_bytes": sum(value[0] for value in expected.values()),
            "inf_files": len(infs), "cat_files": sum(path.lower().endswith(".cat") for path in expected),
            "directory": folder, "directory_files": sum(not folder or path.startswith(folder + "/") for path in expected)}
        (root / f"{name}.manifest.json").write_text(json.dumps(expected, ensure_ascii=False, indent=2), encoding="utf-8")
    (root / "environment.json").write_text(json.dumps(metadata, ensure_ascii=False, indent=2), encoding="utf-8")
    samples = []
    validation = []
    fields = ["package", "operation", "round", "backend", "wall_ms", "operation_ms", "initialization_ms", "total_ms"]
    with (root / "samples.csv").open("w", newline="", encoding="utf-8") as sample_file:
        writer = csv.DictWriter(sample_file, fieldnames=fields)
        writer.writeheader()
        for name, package in metadata["packages"].items():
            expected = json.loads((root / f"{name}.manifest.json").read_text(encoding="utf-8"))
            expected = {path: tuple(value) for path, value in expected.items()}
            archive = Path(package["archive"])
            source = root / "reference" / name
            for operation in operations:
                repeats = arguments.compression_repeats if operation == "compress" else arguments.repeats
                warmups = 1 if operation == "compress" else arguments.warmups
                baseline_index = None
                print(f"START {name} {operation}: {warmups} warmup(s), {repeats} measured pairs", flush=True)
                for round_number in range(-warmups, repeats):
                    order = ["main", "bit7z"] if round_number % 2 == 0 else ["bit7z", "main"]
                    for backend in order:
                        work = root / "scratch" / f"{name}-{operation}-{backend}"
                        work.mkdir(parents=True, exist_ok=False)
                        output = work / ("output.7z" if operation == "compress" else "output.index" if operation == "index" else "extracted")
                        if operation == "index":
                            command = [binaries[backend] / "DriverIndexer.exe", "--language", "en", "index", archive, output]
                        else:
                            command = [binaries[backend] / "archive_backend.exe", operation,
                                       source if operation == "compress" else archive, output]
                            if operation == "directory":
                                command.append(package["directory"])
                        result, wall_ms = run(command)
                        (root / "last-output.log").write_bytes(result.stdout + b"\n" + result.stderr)
                        timings = {} if operation == "index" else json.loads(result.stdout)
                        if round_number >= 0:
                            sample = {"package": name, "operation": operation, "round": round_number,
                                      "backend": backend, "wall_ms": wall_ms, **timings}
                            samples.append(sample)
                            writer.writerow(sample)
                            sample_file.flush()
                        # Index determinism is checked on EVERY sample. Large-file
                        # hashing / decompression is only performed in warmup, untimed.
                        if operation == "index":
                            index = json.loads(output.read_text(encoding="utf-8-sig"))
                            if baseline_index is None:
                                baseline_index = index
                                (root / f"{name}.index.json").write_text(json.dumps(index, ensure_ascii=False, indent=2), encoding="utf-8")
                            if index != baseline_index:
                                raise RuntimeError(f"Index output mismatch: {name} / {backend} / {round_number}")
                        elif round_number < 0 and operation != "metadata":
                            if operation == "compress":
                                run([oracle, "t", output, "-p"])
                                extracted = work / "oracle-extracted"
                                run([oracle, "x", output, f"-o{extracted}", "-y", "-p"])
                                actual = manifest(extracted)
                                selected = expected
                                validation.append({"package": name, "backend": backend, "operation": operation,
                                                   "output_archive_bytes": output.stat().st_size})
                            else:
                                actual = manifest(output)
                                selected = {path: value for path, value in expected.items()
                                            if operation == "full"
                                            or operation == "inf_cat" and path.lower().endswith((".inf", ".cat"))
                                            or operation == "directory" and (not package["directory"] or path.startswith(package["directory"] + "/"))}
                            if actual != selected:
                                raise RuntimeError(f"Extracted file paths/sizes/hashes mismatch: {name} / {operation} / {backend}")
                        remove_owned(work)
                grouped = {}
                for backend in ["main", "bit7z"]:
                    values = [s["wall_ms"] for s in samples if s["package"] == name and s["operation"] == operation and s["backend"] == backend]
                    grouped[backend] = round(statistics.median(values), 3)
                print(f"DONE {name} {operation}: median wall ms {grouped}", flush=True)
    summary = []
    for name in metadata["packages"]:
        for operation in operations:
            row = {"package": name, "operation": operation}
            for backend in ["main", "bit7z"]:
                group = [s for s in samples if s["package"] == name and s["operation"] == operation and s["backend"] == backend]
                row[backend] = {"samples": len(group)}
                for metric in ["wall_ms", "operation_ms", "initialization_ms", "total_ms"]:
                    values = [s[metric] for s in group if metric in s]
                    if values:
                        row[backend][metric] = {"median": statistics.median(values), "p10": percentile(values, .1),
                                                "p90": percentile(values, .9), "min": min(values), "max": max(values)}
            old = row["main"]["wall_ms"]["median"]
            new = row["bit7z"]["wall_ms"]["median"]
            row["wall_reduction_percent"] = (old - new) / old * 100
            row["wall_speedup"] = old / new
            if operation != "index":
                old = row["main"]["operation_ms"]["median"]
                new = row["bit7z"]["operation_ms"]["median"]
                row["operation_reduction_percent"] = (old - new) / old * 100
            summary.append(row)
    (root / "summary.json").write_text(json.dumps({"results": summary, "compression_validation": validation}, ensure_ascii=False, indent=2), encoding="utf-8")
    print(f"COMPLETE: {len(samples)} measurements; results in {root}", flush=True)


if __name__ == "__main__":
    main()
