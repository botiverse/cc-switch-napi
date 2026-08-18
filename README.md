# @botiverse/cc-switch

Native Node.js bindings for the provider-management core of
[CC Switch CLI](https://github.com/SaladDay/cc-switch-cli), built with
[napi-rs](https://napi.rs/).

The source repository is private while the API is being stabilized. Release
automation is configured to publish the package from GitHub Releases; no npm
package is published merely by pushing a commit.

## Supported applications

- Claude Code
- Codex
- Gemini
- OpenCode
- Hermes
- OpenClaw

## API

```ts
import { CcSwitch } from '@botiverse/cc-switch'

const ccSwitch = new CcSwitch()

ccSwitch.supportedApps()
ccSwitch.listProviders('claude')
ccSwitch.currentProvider('claude')
ccSwitch.addProvider('claude', provider)
ccSwitch.updateProvider('claude', provider)
ccSwitch.duplicateProvider('claude', 'provider-id')
ccSwitch.switchProvider('claude', 'provider-id')
ccSwitch.deleteProvider('claude', 'provider-id')
ccSwitch.importLiveConfig('opencode')
ccSwitch.removeFromLiveConfig('opencode', 'provider-id')
ccSwitch.setDefaultProvider('openclaw', 'provider-id', 'model-id')
ccSwitch.readLiveSettings('codex')
ccSwitch.syncCurrentToLive()
ccSwitch.close()
```

`currentProvider()` is meaningful for Claude, Codex, Gemini, and Hermes. The
additive OpenCode and OpenClaw stores intentionally return `null`; use their
live settings/default-model operations for active configuration.

Provider values use the JSON shape supported by the pinned CC Switch revision:

```ts
type Provider = {
  id: string
  name: string
  settingsConfig: unknown
  websiteUrl?: string
  category?: string
  createdAt?: number
  sortIndex?: number
  notes?: string
  meta?: ProviderMeta
  icon?: string
  iconColor?: string
  inFailoverQueue?: boolean
}
```

Unknown provider or metadata fields are ignored by the pinned upstream
deserializer. Update the vendored revision and these declarations together
before relying on newly added upstream fields.

## Storage and safety

CC Switch stores state in `~/.cc-switch` by default. Set
`CC_SWITCH_CONFIG_DIR` **before** creating `CcSwitch` to isolate its database.
Live application paths follow the upstream variables, including
`CLAUDE_CONFIG_DIR` and `CODEX_HOME`.

Those paths are resolved from process-global environment and settings. To
prevent stale in-memory snapshots from overwriting each other, the binding
allows only one active `CcSwitch` instance per process. Reuse it for all
operations, and call `close()` when finished to release the native store and
allow a new instance. Closing is required before synchronously deleting the
store on Windows; garbage collection also releases an unclosed instance
eventually. Do not change path-related environment variables while an instance
is active. External processes must also avoid writing the same store while the
instance is active; the in-memory upstream state is refreshed only at
construction. The same caution applies to external writers of live app
configuration files while switch/sync operations are running.

Provider switching writes the selected application's live configuration when
that application is already initialized (for example, when its config
directory exists). This preserves upstream's safe default of not creating
new application config files unexpectedly. `syncCurrentToLive()` is the
explicit force-sync operation. Use an isolated HOME/config environment in
tests. The constructor opens CC Switch storage but does not import live
configuration implicitly.

All bindings are synchronous because they preserve the upstream service API.
Calls can perform SQLite and filesystem I/O (and a switch may coordinate a
running proxy), so invoke them from a Node worker thread when event-loop
latency matters.

Browser and WASI builds are not supported by this native, filesystem-backed
binding.

Release artifacts cover macOS arm64/x64, Windows x64, and Linux arm64/x64
glibc plus Linux x64 musl.

## Publishing

The release workflow follows the napi-rs package-template flow: each supported
target is built independently, the native artifacts are assembled into scoped
platform packages, `napi prepublish` publishes those packages, and npm then
publishes `@botiverse/cc-switch` with matching optional dependencies.

Publishing only runs for a published GitHub Release whose tag is exactly
`v<package.json version>`. The repository must provide an npm automation token
as the `NPM_TOKEN` Actions secret; npm provenance is enabled for the workflow.
Normal pushes and pull requests build and test artifacts but never publish.

## Development

```bash
corepack yarn install
corepack yarn build:debug
corepack yarn test
cargo fmt --check
cargo clippy --all-targets -- -D warnings
```

The upstream Rust crate is vendored at `vendor/cc-switch-cli` and pinned in
`vendor/cc-switch-cli/UPSTREAM.md`.

## License

MIT. The vendored CC Switch source remains under its upstream MIT notice in
`vendor/cc-switch-cli/LICENSE`.
