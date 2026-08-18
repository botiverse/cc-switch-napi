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

export { CcSwitch } from './index.js'
