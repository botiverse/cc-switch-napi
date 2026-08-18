const { mkdirSync, mkdtempSync } = require('node:fs')
const { tmpdir } = require('node:os')
const { join } = require('node:path')
const { createRequire } = require('node:module')

const sandbox = mkdtempSync(join(tmpdir(), 'cc-switch-published-'))
process.env.CC_SWITCH_CONFIG_DIR = join(sandbox, 'cc-switch')
process.env.CLAUDE_CONFIG_DIR = join(sandbox, 'claude')
process.env.CODEX_HOME = join(sandbox, 'codex')
process.env.HOME = join(sandbox, 'home')
process.env.XDG_CONFIG_HOME = join(sandbox, 'xdg-config')
process.env.XDG_RUNTIME_DIR = join(sandbox, 'xdg-runtime')
process.env.XDG_STATE_HOME = join(sandbox, 'xdg-state')
mkdirSync(process.env.CLAUDE_CONFIG_DIR, { recursive: true })

const requireFromInstall = createRequire(join(process.cwd(), 'package.json'))
const { CcSwitch } = requireFromInstall('@botiverse/cc-switch')
const client = new CcSwitch()

try {
  const apps = client.supportedApps()
  if (apps.join(',') !== 'claude,codex,gemini,opencode,hermes,openclaw') {
    throw new Error(`Unexpected supported apps: ${apps.join(',')}`)
  }
  if (!Array.isArray(client.listProviders('claude'))) {
    throw new Error('listProviders() did not return an array')
  }
} finally {
  client.close()
}
