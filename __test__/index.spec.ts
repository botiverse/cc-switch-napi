import test from 'ava'
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { CcSwitch } from '../index.js'

const sandbox = mkdtempSync(join(tmpdir(), 'cc-switch-napi-'))
process.env.CC_SWITCH_CONFIG_DIR = join(sandbox, 'cc-switch')
process.env.CLAUDE_CONFIG_DIR = join(sandbox, 'claude')
process.env.CODEX_HOME = join(sandbox, 'codex')
process.env.HOME = join(sandbox, 'home')
process.env.XDG_CONFIG_HOME = join(sandbox, 'xdg-config')
process.env.XDG_RUNTIME_DIR = join(sandbox, 'xdg-runtime')
process.env.XDG_STATE_HOME = join(sandbox, 'xdg-state')

// Rust's dirs::home_dir() ignores HOME on Windows. Use CC Switch's supported
// per-application overrides so native MCP/Skills writes remain sandboxed on
// every host without relying on platform-specific home-directory behavior.
mkdirSync(process.env.CC_SWITCH_CONFIG_DIR, { recursive: true })
writeFileSync(
  join(process.env.CC_SWITCH_CONFIG_DIR, 'settings.json'),
  JSON.stringify({
    claudeConfigDir: join(sandbox, 'claude'),
    codexConfigDir: join(sandbox, 'codex'),
    geminiConfigDir: join(sandbox, 'gemini'),
    opencodeConfigDir: join(sandbox, 'opencode'),
    hermesConfigDir: join(sandbox, 'hermes'),
  }),
)

// Upstream deliberately skips live writes for applications that have not
// been initialized. Creating the isolated config directory opts this test's
// fake Claude installation into live synchronization.
mkdirSync(process.env.CLAUDE_CONFIG_DIR, { recursive: true })

const client = new CcSwitch()

test.after.always(() => {
  client.close()
  rmSync(sandbox, { recursive: true, force: true })
})

test.serial('reports every upstream provider application', (t) => {
  const apps = client.supportedApps()
  t.deepEqual(apps, ['claude', 'codex', 'gemini', 'opencode', 'hermes', 'openclaw'])
  for (const app of apps) {
    t.true(Array.isArray(client.listProviders(app)))
  }
  t.is(client.currentProvider('openclaw'), null)
})

test.serial('rejects a second active instance to prevent stale-state writes', (t) => {
  t.throws(() => new CcSwitch(), {
    message: /Only one CcSwitch instance/,
  })
})

test.serial('rejects an unsupported application id', (t) => {
  const error = t.throws(() => client.listProviders('unsupported' as never))
  t.regex(error.message, /Unsupported app id/)
  t.is((error as { code?: string }).code, 'InvalidArg')
})

test.serial('adds and switches a Claude provider in the isolated live config', (t) => {
  const provider = {
    id: 'test-provider',
    name: 'Test Provider',
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: 'https://example.invalid',
        ANTHROPIC_AUTH_TOKEN: 'test-token',
      },
    },
  }

  t.true(client.addProvider('claude', provider))
  t.deepEqual(
    client.listProviders('claude').find((entry) => entry.id === provider.id),
    {
      ...provider,
      inFailoverQueue: false,
    },
  )

  const updatedProvider = {
    ...provider,
    name: 'Updated Test Provider',
  }
  t.true(client.updateProvider('claude', updatedProvider))
  t.is(client.listProviders('claude').find((entry) => entry.id === provider.id)?.name, updatedProvider.name)

  client.switchProvider('claude', provider.id)
  t.is(client.currentProvider('claude'), provider.id)

  const live = JSON.parse(readFileSync(join(sandbox, 'claude', 'settings.json'), 'utf8'))
  t.is(live.env.ANTHROPIC_BASE_URL, 'https://example.invalid')
  t.is(live.env.ANTHROPIC_AUTH_TOKEN, 'test-token')
  const liveSettings = client.readLiveSettings('claude') as { env: Record<string, string> }
  t.is(liveSettings.env.ANTHROPIC_BASE_URL, 'https://example.invalid')
  client.syncCurrentToLive()
  t.is(client.importLiveConfig('claude'), 0)

  const duplicate = client.duplicateProvider('claude', provider.id)
  t.is(duplicate.id, 'test-provider-copy')
  t.is(duplicate.name, 'Updated Test Provider copy')

  client.switchProvider('claude', duplicate.id)
  t.throws(() => client.removeFromLiveConfig('claude', duplicate.id), {
    message: /Only additive-mode apps/,
  })
  t.throws(() => client.setDefaultProvider('claude', duplicate.id), {
    message: /Only Hermes and OpenClaw/,
  })
  client.deleteProvider('claude', provider.id)
  t.false(client.listProviders('claude').some((entry) => entry.id === provider.id))
})

test.serial('exposes default import and non-sensitive common-config operations', (t) => {
  const snippet = client.extractCommonConfigFromSettings('claude', {
    env: {
      ANTHROPIC_BASE_URL: 'https://example.invalid',
      ANTHROPIC_AUTH_TOKEN: 'secret',
      KEEP_ME: 'safe',
    },
    theme: 'dark',
  })

  t.deepEqual(JSON.parse(snippet), {
    env: { KEEP_ME: 'safe' },
    theme: 'dark',
  })

  client.setCommonConfig('claude', snippet)
  client.clearCommonConfig('claude')
  t.false(client.importDefaultConfig('opencode'))
})

test.serial('manages unified MCP registry and application projection', (t) => {
  t.deepEqual(client.supportedMcpApps(), ['claude', 'codex', 'gemini', 'opencode', 'hermes'])

  client.upsertMcpServer({
    id: 'local-echo',
    name: 'Local Echo',
    server: {
      command: 'node',
      args: ['echo.js'],
      env: { TEST_MODE: '1' },
    },
    apps: {},
    tags: ['test'],
  })

  t.is(client.listMcpServers()['local-echo'].name, 'Local Echo')
  t.throws(() => client.toggleMcpApp('local-echo', 'openclaw' as never, true), {
    message: /not supported for OpenClaw/,
  })
  client.toggleMcpApp('local-echo', 'claude', true)
  t.true(client.listMcpServers()['local-echo'].apps.claude)

  const claudeMcp = JSON.parse(readFileSync(join(sandbox, 'claude.json'), 'utf8'))
  t.is(claudeMcp.mcpServers['local-echo'].command, 'node')

  t.true(client.setMcpApps('local-echo', {}))
  t.false(client.listMcpServers()['local-echo'].apps.claude)
  t.true(client.deleteMcpServer('local-echo'))
  t.false('local-echo' in client.listMcpServers())
})

test.serial('imports, projects, configures, and uninstalls a local Skill', (t) => {
  t.deepEqual(client.supportedSkillApps(), ['claude', 'codex', 'gemini', 'opencode', 'hermes'])
  client.setSkillSyncMethod('copy')
  t.is(client.skillSyncMethod(), 'copy')
  t.throws(() => client.syncSkillsToLive('openclaw' as never), {
    message: /not supported for OpenClaw/,
  })

  const source = join(sandbox, 'claude', 'skills', 'local-demo')
  mkdirSync(source, { recursive: true })
  writeFileSync(
    join(source, 'SKILL.md'),
    '---\nname: Local Demo\ndescription: Isolated SDK test skill\n---\n\n# Local Demo\n',
  )

  const unmanaged = client.scanUnmanagedSkills().find((skill) => skill.directory === 'local-demo')
  t.truthy(unmanaged)
  t.true(unmanaged?.foundIn.includes('claude'))

  const [imported] = client.importSkills([{ directory: 'local-demo', apps: { claude: true } }])
  t.is(imported.directory, 'local-demo')
  t.true(imported.apps.claude)
  t.true(existsSync(join(sandbox, 'cc-switch', 'skills', 'local-demo', 'SKILL.md')))

  client.toggleSkillApp('local-demo', 'codex', true)
  t.true(existsSync(join(sandbox, 'codex', 'skills', 'local-demo', 'SKILL.md')))
  t.true(client.setSkillApps('local-demo', { claude: true }))
  t.false(existsSync(join(sandbox, 'codex', 'skills', 'local-demo')))
  client.syncSkillsToLive('claude')
  t.true(client.listSkills().some((skill) => skill.directory === 'local-demo'))

  client.uninstallSkill('local-demo')
  t.false(client.listSkills().some((skill) => skill.directory === 'local-demo'))
  t.false(existsSync(join(sandbox, 'cc-switch', 'skills', 'local-demo')))
})

test.serial('runs network-backed Skill APIs on the native async runtime', async (t) => {
  await t.throwsAsync(() => client.installSkill('', 'claude'), {
    message: /Skill/,
  })
})

test.serial('releases the store and instance slot explicitly', (t) => {
  client.close()
  t.throws(() => client.listProviders('claude'), {
    message: /has been closed/,
  })

  const replacement = new CcSwitch()
  replacement.close()
  t.pass()
})
