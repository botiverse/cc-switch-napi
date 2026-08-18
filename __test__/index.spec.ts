import test from 'ava'
import { mkdirSync, mkdtempSync, readFileSync, rmSync } from 'node:fs'
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

test.serial('releases the store and instance slot explicitly', (t) => {
  client.close()
  t.throws(() => client.listProviders('claude'), {
    message: /has been closed/,
  })

  const replacement = new CcSwitch()
  replacement.close()
  t.pass()
})
