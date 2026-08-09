# Contributing

Bug reports and pull requests are welcome.

## Licence

This project is MIT and stays MIT. Contributions are accepted under the same
terms — there is no contributor licence agreement to sign, because there is no
plan to relicense the core.

Sign your commits off to confirm you have the right to submit the work:

```bash
git commit -s
```

That adds a `Signed-off-by` line, which is the [Developer Certificate of
Origin](https://developercertificate.org/) — you are stating the contribution is
yours to give, nothing more.

## Before you open a pull request

Both of these must be clean:

```bash
cd src-tauri && cargo check     # Rust
bunx tsc --noEmit               # TypeScript
```

Read [docs/PLAN.md](docs/PLAN.md) first if you are changing behaviour. It records
the decisions **and what was rejected**, which saves proposing something that was
already considered and turned down for a reason.

## What this project cares about

**Failures must say which link broke.** This is a rewrite of a version where
almost every fault presented as silence: a listener that played nothing, a key
that did nothing, an installer that reported success over a dead install. A
change that can fail quietly needs a counter, a log line, or something on screen
that names it — that is not polish here, it is the point.

**The real-time audio callbacks allocate nothing, lock nothing, and log
nothing.** A `println!` takes the stdout lock and is audible as a crackle. Work
belongs on the worker threads.

**Comments explain why, not what.** The code says what it does. The valuable
thing is the constraint that made it that way — which Windows behaviour, which
trap, which alternative was tried and failed.
