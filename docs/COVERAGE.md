# Binding coverage

The package binds the provider-switching, unified MCP, and managed Skills data
planes of the vendored CC Switch revision. It does not attempt to expose every
feature of the full desktop/TUI application.

## Bound

| Capability                       | N-API                                                                                            |
| -------------------------------- | ------------------------------------------------------------------------------------------------ |
| Supported apps                   | `supportedApps`                                                                                  |
| List/current provider            | `listProviders`, `currentProvider`                                                               |
| Add/update/duplicate/delete      | `addProvider`, `updateProvider`, `duplicateProvider`, `deleteProvider`                           |
| Switch live provider             | `switchProvider`                                                                                 |
| Import live/default config       | `importLiveConfig`, `importDefaultConfig`                                                        |
| Additive config removal/default  | `removeFromLiveConfig`, `setDefaultProvider`                                                     |
| Read/sync live settings          | `readLiveSettings`, `syncCurrentToLive`                                                          |
| Common config extraction/storage | `extractCommonConfig`, `extractCommonConfigFromSettings`, `setCommonConfig`, `clearCommonConfig` |
| Safe native lifecycle            | constructor, `close`                                                                             |
| MCP registry CRUD                | `listMcpServers`, `upsertMcpServer`, `deleteMcpServer`                                           |
| MCP app matrix/live projection   | `toggleMcpApp`, `setMcpApps`, `syncMcpToLive`, `importMcpFromLive`                               |
| Installed Skills                 | `listSkills`, `installSkill`, `uninstallSkill`                                                   |
| Skill app matrix/live projection | `toggleSkillApp`, `setSkillApps`, `syncSkillsToLive`                                             |
| Unmanaged Skill import           | `scanUnmanagedSkills`, `importSkills`                                                            |
| Skill repositories/sync mode     | `listSkillRepos`, `upsertSkillRepo`, `removeSkillRepo`, `skillSyncMethod`, `setSkillSyncMethod`  |
| Skill discovery/search/update    | `discoverSkills`, `searchSkills`, `checkSkillUpdates`, `updateSkills`                            |

These methods cover the upstream `ProviderService`, `McpService`, and core
`SkillService` state transitions required to persist and synchronize those
three data planes. MCP and Skills target Claude, Codex, Gemini, OpenCode, and
Hermes; the pinned upstream does not support projecting either into OpenClaw.

## Available through existing objects

- Provider sort order is part of `Provider.sortIndex` and can be changed with
  `updateProvider`; the upstream bulk sort helper is not exposed separately.
- Custom endpoints live in `Provider.meta.custom_endpoints` and can be managed
  by reading/updating the provider object; timestamp convenience helpers are
  not exposed separately.
- Provider-key validation for Hermes/OpenClaw runs automatically in
  `addProvider`; low-level key helper functions are implementation details.

## Not bound

The following are adjacent CC Switch product features, not provider switching
state transitions, and remain outside this package's current contract:

- endpoint speed tests and streaming health checks;
- remote model discovery;
- quota and usage-script execution;
- proxy lifecycle, failover routing, sessions, OAuth, prompts, WebDAV/S3 sync,
  daemon management, and desktop/TUI commands;
- CLI-only provider templates and standalone export formatting.

Adding one of these requires a separate typed API and async/cancellation design;
it should not be smuggled into the synchronous switching surface.

## Verification

- Source tests run against isolated HOME/config directories and never touch a
  developer's real application configuration. They verify MCP registry/live
  projection plus local Skill scan/import/app projection/uninstall.
- Release CI builds six native targets and executes tests on native runners.
- After publishing, release CI installs the exact npm version on Linux, macOS,
  and Windows, loads the selected native package, opens an isolated store, and
  verifies discovery/listing before closing it.
- `@botiverse/cc-switch@0.1.1` was installed from npm into a clean directory on
  macOS arm64; native loading, store creation, `supportedApps`, provider listing,
  current-provider reading, and `close` all passed.

The exact vendored revision and license provenance are recorded in
[`vendor/cc-switch-cli/UPSTREAM.md`](../vendor/cc-switch-cli/UPSTREAM.md).
