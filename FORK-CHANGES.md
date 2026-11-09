# thenvoi/copilot-sdk fork changes

Rev-pinned fork of [github/copilot-sdk](https://github.com/github/copilot-sdk)
consumed by Band ([thenvoi/tjam](https://github.com/thenvoi/tjam)) as the
`github-copilot-sdk` Rust crate. The fork changes only the Rust client library
linked into Band; users continue installing and running the stock Copilot CLI.

`main` mirrors upstream. The default branch `jam-rustls-transport` is upstream
`main` plus the fork-only commits below, kept as a linear history: every
fork-only commit has exactly one parent.

## 1. rustls TLS on every platform

Band bans `native-tls` and OpenSSL. Upstream uses rustls only on macOS. On
Linux and Windows it hard-codes `native-tls` on the request-handler `reqwest`
and `tokio-tungstenite` dependencies and on the build-script `ureq` download,
and on Linux the `runtime` feature requires OpenSSL. Build dependencies are
part of every consumer's resolved graph, and Cargo feature unification lets a
dependency's TLS features switch every `reqwest` client in the consumer binary
to OpenSSL.

- Request-handler forwarding selects its TLS backend by feature on every
  platform, extending [github/copilot-sdk#2811](https://github.com/github/copilot-sdk/pull/2811):
  - `rustls` (default) uses rustls with the AWS-LC provider.
  - `rustls-no-provider` uses rustls with the process-wide rustls
    `CryptoProvider` that the consumer installs. TLS forwarding returns an
    error if no provider is installed.
  - `native-tls` remains an opt-in alternative. When both are enabled, rustls
    takes precedence.
  - `network-forwarding` alone forwards plaintext only; it never enables TLS
    implicitly.
- The build script downloads the CLI with ureq's rustls stack and the platform
  verifier on every host OS, selecting the ring provider explicitly. Archive
  hashes use ring on every host except Windows. The build graph contains no
  AWS-LC, `native-tls`, or OpenSSL.

Band uses `default-features = false, features = ["runtime", "rustls-no-provider"]`
and installs ring through its shared TLS owner before constructing SDK clients.

The weekly validation compiles that provider-neutral configuration and inspects
the normal and build dependency graphs for every supported target
(`x86_64` and `aarch64` on `unknown-linux-gnu`, `apple-darwin`, and
`pc-windows-msvc`). It fails if `native-tls` or `openssl-sys` appears with
default features or with `--no-default-features --features rustls`, and if
AWS-LC, `native-tls`, or `openssl-sys` appears with
`--no-default-features --features runtime,rustls-no-provider`.

## 2. Transport-closure observation and cleanup

Copilot's native child can exit while its Node launcher remains alive. Without
an observable transport state, Band kept reporting a dead runtime as live and
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
- Crate version stamp: Band pins by git revision, so the fork keeps upstream's
  `0.0.0-dev` package version.
- Patched ZIP extraction: upstream now depends on `zip` 9.1 and reads archive
  entry names through its fallible API in build-time archive inspection and
  Windows CLI extraction.

`rust/Cargo.lock` is not part of the patch set. Consumers resolve from their
own lockfile, and leaving it untouched keeps upstream lockfile churn from
breaking the weekly replay.

## Weekly upstream replay

`.github/workflows/jam-rebase-rustls-transport.yml` runs every Monday and on
demand. It derives every fork-only commit from the default branch and refuses
to continue if any of them is a merge or root commit. It then cherry-picks the
commits in order onto current `github/copilot-sdk` `main`, compares the
resulting tree with `jam-rustls-transport-latest`, and, when the tree differs,
validates formatting, compilation (including the provider-neutral
configuration), SDK unit tests, archive acquisition, session shutdown, and TLS
dependency graphs before updating the generated branch with `--force-with-lease`.

Validation generates the upstream version snapshots from the pinned release's
checksums and caches the real Ubuntu CLI and runtime archives through GitHub's
release API. Every build configuration reuses those inputs. The SDK verifies
their hashes before use. The generated snapshots and cache are not committed.

If planning or replay fails, use the manual recovery below. If release
preparation or validation fails, inspect that failure and keep the generated
branch unchanged until the required inputs and checks succeed.
When the staged branch advances, Band's `fork-freshness` workflow files an issue
until `Cargo.toml` and `Cargo.lock` pin the new revision.

### Manual recovery

1. Record the failing workflow's upstream target and the fork-only commit list
   (`git rev-list --reverse jam-rustls-transport --not upstream/main`).
2. Start a candidate branch at the latest `upstream/main` and replay the
   fork-only commits in order. Drop any merge commit from the list: the
   commits it brought in already appear there individually. Carry any conflict
   resolution the merge recorded into the replayed commit it belongs to.
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
   `jam-rustls-transport`: the workflow rejects merge commits in the fork-only
   history. Land pull requests into it with "Rebase and merge".

## Upgrade procedure

1. Inspect the latest successful `jam-rebase-rustls-transport` run and its
   staged SHA.
2. Point Band's `github-copilot-sdk` git `rev` at that SHA, keeping
   `default-features = false` and enabling `runtime` and `rustls-no-provider`.
   Keep Band's shared ring provider installed before constructing SDK clients.
3. Run `cargo update -p github-copilot-sdk`, the Copilot host tests, and Band's
   full verification gate (`cargo deny` included).
4. Recheck teardown after native-child loss. Upstream `Client::stop()` waits up
   to 10 seconds for an owned stdio child to exit after stdin EOF, which equals
   Band's Copilot `TEARDOWN_TIMEOUT`; a lingering Node launcher can therefore
   surface as a Band teardown timeout. The fork does not change that wait.
5. Update Band's fork comment and changelog if upstream behavior changed.
6. Remove the fork pin once a released upstream crate contains all required patches.
