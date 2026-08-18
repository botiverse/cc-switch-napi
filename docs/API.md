# API reference

`@botiverse/cc-switch` exports one stateful native class, `CcSwitch`, plus the
TypeScript types from `api.d.ts`.

## Lifecycle

### `new CcSwitch()`

Opens the CC Switch store. Set `CC_SWITCH_CONFIG_DIR` and app-specific path
variables before construction. Only one instance may be active in a process.

### `close(): void`

Closes SQLite resources and releases the process-wide instance slot. Always
call it in `finally`; this is required before deleting the store on Windows.

## Discovery

### `supportedApps(): AppId[]`

Returns `claude`, `codex`, `gemini`, `opencode`, `hermes`, and `openclaw`.

### `listProviders(app: AppId): Provider[]`

Returns the stored provider snapshots for one application.

### `currentProvider(app: AppId): string | null`

Returns the selected provider for Claude, Codex, Gemini, and Hermes. OpenCode
and OpenClaw are additive-mode applications and return `null`.

## Provider management

### `addProvider(app, provider): boolean`

Validates and stores a provider. The first switch-mode provider becomes current.

### `updateProvider(app, provider): boolean`

Replaces a stored provider with the same `id`. If it is active, upstream live
configuration synchronization rules are applied.

### `duplicateProvider(app, sourceId, override?): Provider`

Copies a provider with a collision-free ID and optional full provider override.

### `switchProvider(app, providerId): void`

Selects a provider and writes its live configuration when the target app has
already been initialized.

### `deleteProvider(app, providerId): void`

Deletes a stored provider. Upstream refuses unsafe deletion of an active or
otherwise protected provider.

## Live configuration

### `importLiveConfig(app): number`

Imports provider entries found in the application's current live config.

### `importDefaultConfig(app): boolean`

Imports current live configuration as the initial `default` provider. Returns
`false` for additive-mode apps or when an existing non-official provider makes
the import unsafe.

### `removeFromLiveConfig(app, providerId): void`

Removes a provider from an additive live config without deleting its stored
snapshot.

### `setDefaultProvider(app, providerId, modelId?): string`

Sets the Hermes or OpenClaw default provider/model and returns the selected
model identifier.

### `readLiveSettings(app): JsonValue`

Reads the application's live provider settings without mutating them.

### `syncCurrentToLive(): void`

Synchronizes the current stored provider state back to live app configs.

## Common configuration

### `extractCommonConfig(app): string`

Extracts a non-sensitive common-config snippet from the current provider.

### `extractCommonConfigFromSettings(app, settings): string`

Extracts the same sanitized snippet from an arbitrary settings object. API keys,
tokens, and provider-specific endpoints are removed by upstream logic.

### `setCommonConfig(app, snippet?): void`

Stores a common-config snippet. Passing `null`, `undefined`, or an empty string
clears it.

### `clearCommonConfig(app): void`

Explicitly clears the common-config snippet.

## MCP servers

MCP uses one unified registry with an application matrix. Claude, Codex,
Gemini, OpenCode, and Hermes are supported; OpenClaw is not an MCP projection
target in the pinned upstream revision.

### `supportedMcpApps(): McpAppId[]`

Returns the supported MCP projection targets.

### `listMcpServers(): Record<string, McpServer>`

Returns the unified MCP registry keyed by server ID.

### `upsertMcpServer(server): void`

Validates and adds or replaces a server, then projects it to every application
enabled in `server.apps`.

### `deleteMcpServer(serverId): boolean`

Deletes a server and removes it from all live configs where it was enabled.

### `toggleMcpApp(serverId, app, enabled): void`

Changes one application flag and immediately adds/removes the live projection.

### `setMcpApps(serverId, apps): boolean`

Atomically replaces all application flags. Returns `false` if the server does
not exist.

### `syncMcpToLive(app?): void`

Reprojects the registry to one application, or all supported applications when
`app` is omitted.

### `importMcpFromLive(app?): number`

Imports live MCP definitions from one application, or every supported
application when omitted, into the unified registry.

## Skills

Skills use `~/.cc-switch/skills` (or the isolated CC Switch config directory)
as their single source of truth and project into Claude, Codex, Gemini,
OpenCode, and Hermes. OpenClaw remains unsupported upstream.

### `supportedSkillApps(): SkillAppId[]`

Returns supported Skill projection targets.

### `listSkills(): InstalledSkill[]`

Lists managed Skills and their application matrices.

### `installSkill(spec, app): Promise<InstalledSkill>`

Resolves an upstream-supported repository/directory spec, downloads it into
the Skills SSOT, and enables it for one application. This is the only
asynchronous method in the current SDK.

### `uninstallSkill(directoryOrId): void`

Removes a managed Skill from every application and the SSOT.

### `toggleSkillApp(directoryOrId, app, enabled): void`

Enables or disables one application projection.

### `setSkillApps(directoryOrId, apps): boolean`

Replaces the complete application matrix. Returns `false` only when the
upstream service reports no update.

### `syncSkillsToLive(app?): void`

Projects enabled Skills to one application, or all supported applications.

### `scanUnmanagedSkills(): UnmanagedSkill[]`

Finds valid `SKILL.md` directories in application/agent locations that are not
yet managed.

### `importSkills(selections): InstalledSkill[]`

Imports unmanaged directories with explicit application matrices into the
SSOT.

### `listSkillRepos()`, `upsertSkillRepo(repo)`, `removeSkillRepo(owner, name)`

Manage repository sources used by Skill discovery and installation.

### `skillSyncMethod(): SkillSyncMethod`

Returns `auto`, `symlink`, or `copy`.

### `setSkillSyncMethod(method): void`

Changes the deployment strategy used for subsequent Skill projection.

### `discoverSkills(forceRefresh?): Promise<DiscoverableSkill[]>`

Discovers installable Skills from enabled repositories, using the upstream
cache unless `forceRefresh` is true.

### `searchSkills(query, limit?, offset?): Promise<SkillSearchResult>`

Searches the skills.sh catalog. The upstream client clamps `limit` to 1–100.

### `checkSkillUpdates(): Promise<SkillUpdateCheckResult>`

Compares installed repository-backed Skills with their remote content and
returns both available updates and per-repository failures.

### `updateSkills(ids): Promise<SkillUpdateBatchResult>`

Updates selected Skill IDs and reports successful installed records alongside
per-Skill failures.

## Execution model

All methods except `installSkill()` are synchronous. They may perform
filesystem and SQLite I/O; use a Node worker thread if latency on the main
event loop matters.
