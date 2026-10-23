"""W1.2e: identical clean-source scale sample and per-seat exit measurements."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import time


def main():
    parser = argparse.ArgumentParser()
    for name in ("worktree", "target", "runs", "prefix", "sha"):
        parser.add_argument("--" + name, required=True)
    args = parser.parse_args()
    worktree, target, runs = (Path(getattr(args, n)).resolve() for n in ("worktree", "target", "runs"))
    prefix = Path(args.prefix).resolve()
    assert subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=worktree, text=True).strip() == args.sha
    assert not subprocess.check_output(["git", "status", "--porcelain"], cwd=worktree)
    runs.mkdir(parents=True, mode=0o700, exist_ok=True)
    assert not list(runs.iterdir())
    env = os.environ.copy()
    env.update(CARGO_TARGET_DIR=str(target), CARGO_INCREMENTAL="0", BBOXY_TEST_CUSTODY="memory",
               CARGO_PROFILE_RELEASE_DEBUG="1", GATE_STAGED="1", GATE_SEATS="967",
               GATE_RATE="32kB/s", GATE_SECONDS="60", GATE_RUN_DIR=str(runs))
    results = []

    def record(stage, command, pid, code, started):
        results.append(dict(source=args.sha, stage=stage, command=command, child_pid=pid,
                            child_exit=code, wall_seconds=time.monotonic() - started))
        prefix.with_name(prefix.name + "-status.log").write_text(json.dumps(results, indent=2) + "\n")
        print(f"{stage}: exit={code}, wall={results[-1]['wall_seconds']:.3f}s", flush=True)
        if code:
            raise SystemExit(code)

    def run(stage, command):
        started = time.monotonic()
        with prefix.with_name(prefix.name + "-" + stage + ".log").open("w") as log:
            log.write(f"SOURCE={args.sha}\nCOMMAND={command!r}\n"); log.flush()
            process = subprocess.Popen(command, cwd=worktree, env=env, stdout=log, stderr=subprocess.STDOUT)
            log.write(f"OWN_CHILD_PID={process.pid}\n"); log.flush()
            code = process.wait()
            log.write(f"CHILD_EXIT={code}\n")
        record(stage, command, process.pid, code, started)

    def build_test(package, name):
        stage = "build-" + name
        command = ["cargo", "test", "--release", "--locked", "-p", package,
                   "--test", name, "--no-run", "--message-format=json"]
        started = time.monotonic()
        executable = None
        with prefix.with_name(prefix.name + "-" + stage + ".log").open("w") as log:
            log.write(f"SOURCE={args.sha}\nCOMMAND={command!r}\n"); log.flush()
            process = subprocess.Popen(command, cwd=worktree, env=env, stdout=subprocess.PIPE,
                                       stderr=log, text=True)
            log.write(f"OWN_CHILD_PID={process.pid}\n"); log.flush()
            for line in process.stdout:
                log.write(line)
                item = json.loads(line)
                if item.get("reason") == "compiler-artifact" and item.get("target", {}).get("name") == name and item.get("executable"):
                    executable = item["executable"]
            code = process.wait()
            log.write(f"CHILD_EXIT={code}\n")
        record(stage, command, process.pid, code, started)
        assert executable
        return executable

    print(f"DRIVER_PID={os.getpid()} SOURCE={args.sha}", flush=True)
    run("firehose-build", ["cargo", "build", "--release", "--locked", "-p", "bboxy-firehose"])
    gate = build_test("bbt-mux", "gates_seats")
    command = [gate, "gate_memory", "--ignored", "--nocapture", "--test-threads=1"]
    started = time.monotonic()
    sample = None
    with prefix.with_name(prefix.name + "-gate.log").open("w") as log, prefix.with_name(prefix.name + "-sample-command.log").open("w") as sample_log:
        log.write(f"SOURCE={args.sha}\nCOMMAND={command!r}\n"); log.flush()
        process = subprocess.Popen(command, cwd=worktree, env=env, stdout=subprocess.PIPE,
                                   stderr=subprocess.STDOUT, text=True)
        log.write(f"OWN_CHILD_PID={process.pid}\n"); log.flush()
        for line in process.stdout:
            log.write(line); log.flush()
            if "GATE resumed staged_children=967" in line:
                assert sample is None
                sample_args = ["/usr/bin/sample", str(process.pid), "10", "1", "-file",
                               str(prefix.with_name(prefix.name + "-sample.log"))]
                sample_log.write(f"SOURCE={args.sha}\nCOMMAND={sample_args!r}\n"); sample_log.flush()
                sample = subprocess.Popen(sample_args, stdout=sample_log, stderr=subprocess.STDOUT)
                sample_log.write(f"OWN_SAMPLE_PID={sample.pid}\n"); sample_log.flush()
        code = process.wait()
        log.write(f"CHILD_EXIT={code}\n"); log.flush()
        assert sample is not None, "gate never reached the measured phase"
        sample_code = sample.wait()
        sample_log.write(f"CHILD_EXIT={sample_code}\n")
        assert sample_code == 0, "native sample failed"
    record("967-sampled", command, process.pid, code, started)
    assert not list(runs.iterdir()), "owned gate did not clean its run"
    latency = build_test("bbt-supervisor", "reaping")
    run("967-exit-latency", [latency, "exit_events_at_machine_scale", "--ignored", "--nocapture", "--test-threads=1"])
    assert not subprocess.check_output(["git", "status", "--porcelain"], cwd=worktree)
    print("PROFILE_DRIVER_EXIT=0", flush=True)


if __name__ == "__main__":
    main()
