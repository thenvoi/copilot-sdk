# thenvoi/copilot-sdk fork changes

Rev-pinned fork of [github/copilot-sdk](https://github.com/github/copilot-sdk)
consumed by [thenvoi/tjam](https://github.com/thenvoi/tjam) as the
`github-copilot-sdk` Rust crate. The fork changes only the Rust client library
linked into Jam; users continue installing and running the stock Copilot CLI.

`main` mirrors upstream. The default branch `jam-rustls-transport` is upstream
`main` plus the fork-only commits below, kept as a linear history.

## 1. rustls-only TLS

Jam bans `native-tls` and OpenSSL. Upstream hard-codes `native-tls` on the
request-handler `reqwest` and `tokio-tungstenite` dependencies and on the
build-script `ureq` download, and Cargo feature unification lets a dependency's
TLS features switch every `reqwest` client in the consumer binary to OpenSSL.

- Backport of [github/copilot-sdk#2811](https://github.com/github/copilot-sdk/pull/2811):
  the request-handler transport gets a default `rustls` feature and an opt-in
  `native-tls` feature instead of a hard-coded backend. Jam builds with
  `default-features = false`, so it must enable `rustls` explicitly.
- The build-time CLI download uses ureq's rustls provider with the platform
  verifier. Upstream #1964 moved it to `native-tls`; build dependencies are part
  of every consumer's resolved graph, so that change reintroduced `native-tls`
  even with `default-features = false`.

The weekly validation fails if `native-tls` or `openssl-sys` reappears in the
`--no-default-features --features rustls` normal or build dependency graph.

## 2. Transport-closure observation and cleanup

Copilot's native child can exit while its Node launcher remains alive. Without
an observable transport state, Jam kept reporting a dead runtime as live and
teardown returned broken-pipe errors after the child was already reaped.

- `Client::is_disconnected()` reports observed EOF, read failure, write
  failure, or explicit transport closure.
- `Client::wait_for_disconnect()` is cancel-safe and returns immediately for
  late waiters.
- A failed write closes the connection and cancels pending requests; requests
  issued after closure fail with a transport error instead of waiting for a
  response that cannot arrive.
- `Client::stop()` skips impossible remote cleanup after confirmed transport
  loss, clears local session routing, and still reaps the owned child. Healthy
  RPC failures remain reported.

The port reuses upstream's connection-closed cancellation token rather than a
second closure signal. Originally reviewed as
[thenvoi/copilot-sdk#1](https://github.com/thenvoi/copilot-sdk/pull/1) on the
v1.0.6-preview.1 base.

## Dropped since the v1.0.6-preview.1 fork

- Spawned `userInput.request` dispatch: upstream now dispatches every inbound
  JSON-RPC request on its own task, so a pending human answer no longer blocks
  session event processing.
- Crate version stamp: Jam pins by git revision, so the fork keeps upstream's
  `0.0.0-dev` package version.

`rust/Cargo.lock` is not part of the patch set. Consumers resolve from their
own lockfile, and leaving it untouched keeps upstream lockfile churn from
breaking the weekly replay.

## Weekly upstream replay

`.github/workflows/jam-rebase-rustls-transport.yml` runs every Monday and on
demand. It derives every fork-only commit from the default branch, replays that
patch set onto current `github/copilot-sdk` `main`, compares the resulting tree
with `jam-rustls-transport-latest`, validates formatting, compilation, SDK unit
tests, and the TLS dependency graph, and force-with-lease updates the generated
branch only when either upstream or the maintained patch set changes.

A red run means upstream drifted under the patch and requires a manual rebase.
When the staged branch advances, tjam's `fork-freshness` workflow files an
issue until `Cargo.toml` and `Cargo.lock` pin the new revision.

### Manual conflict recovery

1. Record the failing workflow's upstream target and fork-only commit list.
2. Start a candidate branch at the latest `upstream/main` and replay those
   commits in order.
3. If upstream now provides a fork capability (for example #2811 merges), omit
   that obsolete patch only after verifying the equivalent public API and its
   tests.
4. Resolve remaining conflicts by preserving current upstream behavior and the
   smallest still-required fork delta, then run the workflow validation commands
   from `rust/`.
5. After review, update both `jam-rustls-transport` and
   `jam-rustls-transport-latest` to the same validated linear history with
   `--force-with-lease`, and dispatch this workflow once. Do not merge a
   recovery branch, and do not use GitHub's merge-commit or "Sync fork" flows on
   `jam-rustls-transport`: a merge commit would itself become a fork-only replay
   input. Land pull requests into it with "Rebase and merge".

## Upgrade procedure

1. Inspect the latest successful `jam-rebase-rustls-transport` run and its
   staged SHA.
2. Point tjam's `github-copilot-sdk` git `rev` at that SHA, keeping
   `default-features = false` and enabling the `rustls` feature.
3. Run `cargo update -p github-copilot-sdk`, the Copilot host tests, and the
   full tjam verification gate (`cargo deny` included).
4. Recheck teardown after native-child loss. Upstream `Client::stop()` now
   waits up to 10 seconds for an owned stdio child to exit after stdin EOF,
   which equals tjam's Copilot `TEARDOWN_TIMEOUT`; a lingering Node launcher
   can therefore surface as a tjam teardown timeout. The fork does not change
   that wait.
5. Update tjam's fork comment and changelog if upstream behavior changed.
6. Remove the fork pin once a released upstream crate contains both patches.
