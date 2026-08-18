export type AppId = 'claude' | 'codex' | 'gemini' | 'opencode' | 'hermes' | 'openclaw'

export type JsonPrimitive = boolean | number | string | null
export type JsonObject = { [key: string]: JsonValue }
export type JsonValue = JsonPrimitive | JsonValue[] | JsonObject

/** Provider metadata fields supported by the vendored CC Switch revision. */
export interface ProviderMeta {
  commonConfigEnabled?: boolean
  codexOfficial?: boolean
  custom_endpoints?: Record<string, JsonValue>
  usage_script?: JsonObject
  endpointAutoSelect?: boolean
  isPartner?: boolean
  partnerPromotionKey?: string
  costMultiplier?: string
  pricingModelSource?: string
  limitDailyUsd?: string
  limitMonthlyUsd?: string
  testConfig?: JsonObject
  proxyConfig?: JsonObject
  apiFormat?: string
  codexChatReasoning?: JsonObject
  impersonateClaudeCode?: boolean
  maxOutputTokens?: number
  promptCacheKey?: string
  promptCacheRouting?: string
  codexFastMode?: boolean
  customUserAgent?: string
  localProxyRequestOverrides?: JsonObject
  authBinding?: JsonObject
  apiKeyField?: string
  isFullUrl?: boolean
  liveConfigManaged?: boolean
  providerType?: string
  githubAccountId?: string
}

/** The provider shape supported by the vendored CC Switch revision. */
export interface Provider {
  id: string
  name: string
  settingsConfig: JsonValue
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

export type McpAppId = Exclude<AppId, 'openclaw'>
export type SkillAppId = Exclude<AppId, 'openclaw'>

export interface McpApps {
  claude?: boolean
  codex?: boolean
  gemini?: boolean
  opencode?: boolean
  hermes?: boolean
}

/** Unified MCP definition persisted by CC Switch. */
export interface McpServer {
  id: string
  name: string
  server: JsonValue
  apps: McpApps
  description?: string
  homepage?: string
  docs?: string
  tags?: string[]
}

export interface SkillApps {
  claude?: boolean
  codex?: boolean
  gemini?: boolean
  opencode?: boolean
  hermes?: boolean
}

export type SkillSyncMethod = 'auto' | 'symlink' | 'copy'

export interface InstalledSkill {
  id: string
  name: string
  description?: string
  directory: string
  repoOwner?: string
  repoName?: string
  repoBranch?: string
  readmeUrl?: string
  apps: SkillApps
  installedAt: number
  contentHash?: string
  updatedAt: number
}

export interface UnmanagedSkill {
  directory: string
  name: string
  description?: string
  foundIn: string[]
  path: string
}

export interface ImportSkillSelection {
  directory: string
  apps?: SkillApps
}

export interface SkillRepo {
  owner: string
  name: string
  branch: string
  enabled: boolean
}

export interface DiscoverableSkill {
  key: string
  name: string
  description: string
  directory: string
  readmeUrl?: string
  installed: boolean
  repoOwner?: string
  repoName?: string
  repoBranch?: string
}

export interface SkillSearchEntry {
  key: string
  name: string
  directory: string
  repoOwner: string
  repoName: string
  repoBranch: string
  installs: number
  readmeUrl?: string
}

export interface SkillSearchResult {
  skills: SkillSearchEntry[]
  totalCount: number
  query: string
}

export interface SkillUpdateInfo {
  id: string
  name: string
  directory: string
  currentHash?: string
  remoteHash: string
}

export interface SkillUpdateCheckResult {
  updates: SkillUpdateInfo[]
  failures: string[]
}

export interface SkillUpdateFailure {
  id: string
  error: string
}

export interface SkillUpdateBatchResult {
  updated: InstalledSkill[]
  failures: SkillUpdateFailure[]
}

export { CcSwitch } from './index.js'
