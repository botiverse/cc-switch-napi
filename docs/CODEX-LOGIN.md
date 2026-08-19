# Codex OAuth login

`@botiverse/cc-switch` exposes the vendored CC Switch Codex OAuth device-code
flow. It is intended for a desktop UI: the native library owns the OAuth
credentials, while JavaScript receives only a short-lived device handoff and
non-sensitive account summaries.

```ts
const ccSwitch = new CcSwitch()
try {
  const login = await ccSwitch.startCodexLogin()
  // Open login.verificationUri in the user's browser and display login.userCode.
  // Poll no faster than login.interval seconds until the call returns an account.
  let account: CodexAccount | null = null
  while (!account) {
    account = await ccSwitch.pollCodexLogin(login.deviceCode)
    if (!account) await new Promise((resolve) => setTimeout(resolve, login.interval * 1000))
  }
  console.log(`Signed in as ${account.login}`)
} finally {
  ccSwitch.close()
}
```

`pollCodexLogin()` returns `null` while the user has not completed the browser
step. The device code expires after `expiresIn` seconds; surface an expiry or
network error and offer a fresh `startCodexLogin()` call. The flow does not
require pasting a browser callback into the application: the user opens the
verification URI and enters the displayed user code.

## Account management

- `codexAuthStatus()` returns authentication state, the default account ID,
  and account summaries.
- `listCodexAccounts()` lists the same summaries.
- `setDefaultCodexAccount(accountId)` changes the default account.
- `removeCodexAccount(accountId)` removes one stored account.
- `logoutCodex()` clears all Codex OAuth accounts and native credentials.

Access and refresh tokens never cross the N-API boundary and are not included
in `CodexLoginStart`, `CodexAccount`, or `CodexAuthStatus`. Keep the native
configuration directory protected by the operating system. Provider switching
(`switchProvider`) and OAuth account management are separate operations: a
provider snapshot does not contain OAuth tokens.
