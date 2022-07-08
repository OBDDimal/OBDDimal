# Continuous Integration

## Checks

The `ci` workflow defined in [`ci.yml`](/.github/workflows/ci.yml) runs on every
pushed commit and for a temporary merge commit in each pull request.

It tests compilation, runs static analysis and runs the unit tests.

All jobs in this workflow can run on any runner, and are currently run on
GitHub provided runners with the `ubuntu-latest` image.

## Benchmarks

The `Benchmark` workflow defined in
[`pr_bench.yml`](/.github/workflows/pr_bench.yml) runs on every pull request.

It benchmarks the performance before and after the PR would be merged.
The target branch benchmark is executed on every run, results are not stored and
reused.
The benchmark results are displayed on a web page hosted using GitHub pages in
this same repository.
The link is `https://ottojo.github.io/OBDDimal/<PR NUMBER>/merge/report/`, the
proper link for each run is also available on the job summary page.

### Runner

The benchmarks are run on a dedicated self-hosted runner, identified by the tags
`[self-hosted, benchmark]`.
Care should be taken that (at least during job execution) the system environment
on the runner stays consistent, which can not be expected using a shared, GitHub
provided runner.

The runner is currently installed on a dedicated server, using [the official
instructions](https://docs.github.com/en/actions/hosting-your-own-runners)
and without any virtualization, containerization or autoscaling, but installed
[as a systemd service](https://docs.github.com/en/actions/hosting-your-own-runners/configuring-the-self-hosted-runner-application-as-a-service).
This is done because it is currently not possible to use a custom docker image
for a ci job, while the runner itself runs in a docker container as well.
This is documented in the corresponding [issue in the actions/runner repo](https://github.com/actions/runner/issues/406).
