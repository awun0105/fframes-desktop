#!/usr/bin/env python3
"""Decides whether a tag may be published as a consumer release, and for which targets.

The release workflow fetches the CI runs of the tag commit with `gh` and passes them in, so
the decision itself runs offline:

- the release policy enables consumer releases and lists `eligible_targets`, each one a
  target the policy builds;
- a `Desktop self-hosted validation` run of the tag commit succeeded as a full run: every
  job ran its full-only step (a push run skips it);
- a `Desktop CI` run of the tag commit on `main` succeeded (it has no push trigger, so this
  is a run dispatched on `main`).

Prints the eligible targets, separated by spaces, or exits non-zero with the reason.
"""
import argparse
import json
import sys
from pathlib import Path

SELF_HOSTED_WORKFLOW = "Desktop self-hosted validation"
DESKTOP_WORKFLOW = "Desktop CI"
# Runs only with the workflow's `full` input, in both jobs.
FULL_ONLY_STEP = "Real worker frame, anchor, crash and restart"


class Refused(Exception):
    pass


def eligible_targets(policy):
    publication = policy["publication"]
    if not publication["consumer_release_enabled"] or not publication["trusted_release_keys_configured"]:
        raise Refused("Consumer release is disabled until trusted signing keys are configured.")
    built = {target["triple"] for target in policy["targets"]}
    eligible = policy.get("eligible_targets", [])
    if not eligible:
        raise Refused("No target is eligible for consumer publication.")
    unknown = sorted(set(eligible) - built)
    if unknown:
        raise Refused(f"Eligible targets the release does not build: {', '.join(unknown)}")
    return list(eligible)


def is_full_run(run, commit):
    jobs = run.get("jobs") or []
    return (
        run.get("workflowName") == SELF_HOSTED_WORKFLOW
        and run.get("headSha") == commit
        and run.get("conclusion") == "success"
        and bool(jobs)
        and all(
            job.get("conclusion") == "success"
            and any(
                step.get("name") == FULL_ONLY_STEP and step.get("conclusion") == "success"
                for step in job.get("steps") or []
            )
            for job in jobs
        )
    )


def is_main_run(run, commit):
    return (
        run.get("workflowName") == DESKTOP_WORKFLOW
        and run.get("headSha") == commit
        and run.get("headBranch") == "main"
        and run.get("conclusion") == "success"
    )


def check(policy, tag_commit, self_hosted_runs, desktop_runs):
    targets = eligible_targets(policy)
    if not any(is_full_run(run, tag_commit) for run in self_hosted_runs):
        raise Refused(f"No successful full {SELF_HOSTED_WORKFLOW} run of {tag_commit}.")
    if not any(is_main_run(run, tag_commit) for run in desktop_runs):
        raise Refused(f"No successful {DESKTOP_WORKFLOW} run of {tag_commit} on main.")
    return targets


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--policy", type=Path, required=True)
    parser.add_argument("--tag-commit", required=True)
    parser.add_argument("--self-hosted-runs", type=Path, required=True, help="JSON list of `gh run view --json` objects")
    parser.add_argument("--desktop-runs", type=Path, required=True, help="JSON list of `gh run view --json` objects")
    args = parser.parse_args(argv)
    try:
        targets = check(
            json.loads(args.policy.read_text()),
            args.tag_commit,
            json.loads(args.self_hosted_runs.read_text()),
            json.loads(args.desktop_runs.read_text()),
        )
    except Refused as refused:
        print(refused, file=sys.stderr)
        raise SystemExit(1)
    return " ".join(targets)


if __name__ == "__main__":
    print(main())
