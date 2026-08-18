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

## Execution model

All methods are synchronous because the pinned provider service is synchronous.
They may perform filesystem and SQLite I/O; use a Node worker thread if latency
on the main event loop matters.
