#![deny(clippy::all)]

use std::{
  str::FromStr,
  sync::atomic::{AtomicBool, Ordering},
};

use cc_switch_lib::{
  AppState, AppType, ImportSkillSelection, McpApps, McpServer, McpService, Provider,
  ProviderService, SkillApps, SkillService,
};
use napi::{Error, Result, Status};
use napi_derive::napi;
use serde_json::Value;

static INSTANCE_ACTIVE: AtomicBool = AtomicBool::new(false);

fn napi_error(error: impl std::fmt::Display) -> Error {
  Error::new(Status::GenericFailure, error.to_string())
}

fn parse_app(app: &str) -> Result<AppType> {
  AppType::from_str(app).map_err(|error| Error::new(Status::InvalidArg, error.to_string()))
}

fn parse_projection_app(app: &str, feature: &str) -> Result<AppType> {
  let parsed = parse_app(app)?;
  if matches!(parsed, AppType::OpenClaw) {
    return Err(Error::new(
      Status::InvalidArg,
      format!("{feature} is not supported for OpenClaw"),
    ));
  }
  Ok(parsed)
}

fn parse_provider(value: Value) -> Result<Provider> {
  serde_json::from_value(value).map_err(|error| {
    Error::new(
      Status::InvalidArg,
      format!("Invalid provider object: {error}"),
    )
  })
}

fn parse_json<T: serde::de::DeserializeOwned>(value: Value, label: &str) -> Result<T> {
  serde_json::from_value(value)
    .map_err(|error| Error::new(Status::InvalidArg, format!("Invalid {label}: {error}")))
}

fn serialize_json<T: serde::Serialize>(value: T) -> Result<Value> {
  serde_json::to_value(value).map_err(napi_error)
}

fn count_to_u32(count: usize, label: &str) -> Result<u32> {
  u32::try_from(count)
    .map_err(|_| Error::new(Status::GenericFailure, format!("{label} count exceeds u32")))
}

/// Stateful access to CC Switch provider storage and live configuration.
///
/// CC Switch reads its storage location from `CC_SWITCH_CONFIG_DIR`. Set that
/// environment variable before constructing this class when isolation is
/// required. Live application locations continue to follow the upstream
/// environment variables such as `CLAUDE_CONFIG_DIR` and `CODEX_HOME`.
/// Path selection is process-global; do not retarget environment variables
/// while an instance is active. Only one instance may exist per process.
#[napi]
pub struct CcSwitch {
  state: Option<AppState>,
}

#[napi]
impl CcSwitch {
  /// Opens the current CC Switch store without importing live configuration.
  #[napi(constructor)]
  pub fn new() -> Result<Self> {
    if INSTANCE_ACTIVE
      .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
      .is_err()
    {
      return Err(Error::new(
        Status::GenericFailure,
        "Only one CcSwitch instance may be active per process".to_string(),
      ));
    }

    match AppState::try_new() {
      Ok(state) => Ok(Self { state: Some(state) }),
      Err(error) => {
        INSTANCE_ACTIVE.store(false, Ordering::Release);
        Err(napi_error(error))
      }
    }
  }

  /// Closes the underlying store and releases the process-wide instance slot.
  ///
  /// Call this before deleting the store directory on Windows. Further method
  /// calls on this object fail; constructing a new instance is allowed.
  #[napi]
  pub fn close(&mut self) {
    self.state.take();
    INSTANCE_ACTIVE.store(false, Ordering::Release);
  }

  /// Returns every application supported by the vendored CC Switch core.
  #[napi(ts_return_type = "Array<import('./api.js').AppId>")]
  pub fn supported_apps(&self) -> Vec<String> {
    AppType::all().map(|app| app.as_str().to_string()).collect()
  }

  /// Lists provider objects using the vendored CC Switch serialized shape.
  #[napi(ts_return_type = "Array<import('./api.js').Provider>")]
  pub fn list_providers(
    &self,
    #[napi(ts_arg_type = "import('./api.js').AppId")] app: String,
  ) -> Result<Vec<Value>> {
    let app = parse_app(&app)?;
    ProviderService::list(self.state()?, app)
      .and_then(|providers| {
        providers
          .into_values()
          .map(serde_json::to_value)
          .collect::<std::result::Result<Vec<_>, _>>()
          .map_err(|source| cc_switch_lib::AppError::JsonSerialize { source })
      })
      .map_err(napi_error)
  }

  /// Returns the current provider id for switch-mode apps and Hermes.
  /// OpenCode/OpenClaw are additive-mode apps and return `null`.
  #[napi]
  pub fn current_provider(
    &self,
    #[napi(ts_arg_type = "import('./api.js').AppId")] app: String,
  ) -> Result<Option<String>> {
    let app = parse_app(&app)?;
    ProviderService::current(self.state()?, app)
      .map(|id| (!id.is_empty()).then_some(id))
      .map_err(napi_error)
  }

  /// Adds a provider using fields supported by the vendored upstream revision.
  #[napi]
  pub fn add_provider(
    &self,
    #[napi(ts_arg_type = "import('./api.js').AppId")] app: String,
    #[napi(ts_arg_type = "import('./api.js').Provider")] provider: Value,
  ) -> Result<bool> {
    ProviderService::add(self.state()?, parse_app(&app)?, parse_provider(provider)?)
      .map_err(napi_error)
  }

  /// Replaces an existing provider by id.
  #[napi]
  pub fn update_provider(
    &self,
    #[napi(ts_arg_type = "import('./api.js').AppId")] app: String,
    #[napi(ts_arg_type = "import('./api.js').Provider")] provider: Value,
  ) -> Result<bool> {
    ProviderService::update(self.state()?, parse_app(&app)?, parse_provider(provider)?)
      .map_err(napi_error)
  }

  /// Duplicates a provider, optionally applying an upstream-shaped override.
  #[napi(ts_return_type = "import('./api.js').Provider")]
  pub fn duplicate_provider(
    &self,
    #[napi(ts_arg_type = "import('./api.js').AppId")] app: String,
    source_id: String,
    #[napi(ts_arg_type = "import('./api.js').Provider | null | undefined")]
    provider_override: Option<Value>,
  ) -> Result<Value> {
    let provider_override = provider_override.map(parse_provider).transpose()?;
    ProviderService::duplicate(
      self.state()?,
      parse_app(&app)?,
      &source_id,
      provider_override,
    )
    .and_then(|provider| {
      serde_json::to_value(provider)
        .map_err(|source| cc_switch_lib::AppError::JsonSerialize { source })
    })
    .map_err(napi_error)
  }

  /// Switches the selected provider and, when initialized, writes its live config.
  #[napi]
  pub fn switch_provider(
    &self,
    #[napi(ts_arg_type = "import('./api.js').AppId")] app: String,
    provider_id: String,
  ) -> Result<()> {
    ProviderService::switch(self.state()?, parse_app(&app)?, &provider_id).map_err(napi_error)
  }

  /// Deletes a provider. CC Switch refuses to delete an active provider.
  #[napi]
  pub fn delete_provider(
    &self,
    #[napi(ts_arg_type = "import('./api.js').AppId")] app: String,
    provider_id: String,
  ) -> Result<()> {
    ProviderService::delete(self.state()?, parse_app(&app)?, &provider_id).map_err(napi_error)
  }

  /// Imports providers from the application's current live configuration.
  #[napi]
  pub fn import_live_config(
    &self,
    #[napi(ts_arg_type = "import('./api.js').AppId")] app: String,
  ) -> Result<u32> {
    ProviderService::import_live_config(self.state()?, parse_app(&app)?)
      .and_then(|count| {
        u32::try_from(count).map_err(|_| {
          cc_switch_lib::AppError::Message("Imported provider count exceeds u32".to_string())
        })
      })
      .map_err(napi_error)
  }

  /// Imports the current live config as the initial `default` provider.
  /// Additive-mode apps return `false`; existing non-official providers are not overwritten.
  #[napi]
  pub fn import_default_config(
    &self,
    #[napi(ts_arg_type = "import('./api.js').AppId")] app: String,
  ) -> Result<bool> {
    ProviderService::import_default_config(self.state()?, parse_app(&app)?).map_err(napi_error)
  }

  /// Removes a provider from an additive live config without deleting it.
  #[napi]
  pub fn remove_from_live_config(
    &self,
    #[napi(ts_arg_type = "import('./api.js').AppId")] app: String,
    provider_id: String,
  ) -> Result<()> {
    ProviderService::remove_from_live_config(self.state()?, parse_app(&app)?, &provider_id)
      .map_err(napi_error)
  }

  /// Sets the default provider/model for Hermes or OpenClaw.
  #[napi]
  pub fn set_default_provider(
    &self,
    #[napi(ts_arg_type = "import('./api.js').AppId")] app: String,
    provider_id: String,
    model_id: Option<String>,
  ) -> Result<String> {
    ProviderService::set_default_model(
      self.state()?,
      parse_app(&app)?,
      &provider_id,
      model_id.as_deref(),
    )
    .map_err(napi_error)
  }

  /// Reads the application's live provider settings without changing them.
  #[napi(ts_return_type = "import('./api.js').JsonValue")]
  pub fn read_live_settings(
    &self,
    #[napi(ts_arg_type = "import('./api.js').AppId")] app: String,
  ) -> Result<Value> {
    ProviderService::read_live_settings(parse_app(&app)?).map_err(napi_error)
  }

  /// Extracts a non-sensitive common-config snippet from the current provider.
  #[napi]
  pub fn extract_common_config(
    &self,
    #[napi(ts_arg_type = "import('./api.js').AppId")] app: String,
  ) -> Result<String> {
    ProviderService::extract_common_config_snippet(self.state()?, parse_app(&app)?)
      .map_err(napi_error)
  }

  /// Extracts a non-sensitive common-config snippet from an arbitrary settings object.
  #[napi]
  pub fn extract_common_config_from_settings(
    &self,
    #[napi(ts_arg_type = "import('./api.js').AppId")] app: String,
    #[napi(ts_arg_type = "import('./api.js').JsonValue")] settings: Value,
  ) -> Result<String> {
    ProviderService::extract_common_config_snippet_from_settings(parse_app(&app)?, &settings)
      .map_err(napi_error)
  }

  /// Stores or clears the app-level common-config snippet used during provider writes.
  #[napi]
  pub fn set_common_config(
    &self,
    #[napi(ts_arg_type = "import('./api.js').AppId")] app: String,
    snippet: Option<String>,
  ) -> Result<()> {
    ProviderService::set_common_config_snippet(self.state()?, parse_app(&app)?, snippet)
      .map_err(napi_error)
  }

  /// Clears the app-level common-config snippet.
  #[napi]
  pub fn clear_common_config(
    &self,
    #[napi(ts_arg_type = "import('./api.js').AppId")] app: String,
  ) -> Result<()> {
    ProviderService::clear_common_config_snippet(self.state()?, parse_app(&app)?)
      .map_err(napi_error)
  }

  /// Writes all currently selected providers back to their live configs.
  #[napi]
  pub fn sync_current_to_live(&self) -> Result<()> {
    ProviderService::sync_current_to_live(self.state()?).map_err(napi_error)
  }

  /// Returns applications that support MCP projection.
  #[napi(ts_return_type = "Array<import('./api.js').McpAppId>")]
  pub fn supported_mcp_apps(&self) -> Vec<String> {
    McpService::supported_mcp_apps()
      .map(|app| app.as_str().to_string())
      .collect()
  }

  /// Lists the unified MCP server registry keyed by server id.
  #[napi(ts_return_type = "Record<string, import('./api.js').McpServer>")]
  pub fn list_mcp_servers(&self) -> Result<Value> {
    McpService::get_all_servers(self.state()?)
      .and_then(|servers| {
        serde_json::to_value(servers)
          .map_err(|source| cc_switch_lib::AppError::JsonSerialize { source })
      })
      .map_err(napi_error)
  }

  /// Adds or replaces an MCP server and projects it to enabled applications.
  #[napi]
  pub fn upsert_mcp_server(
    &self,
    #[napi(ts_arg_type = "import('./api.js').McpServer")] server: Value,
  ) -> Result<()> {
    let server: McpServer = parse_json(server, "MCP server")?;
    McpService::upsert_server(self.state()?, server).map_err(napi_error)
  }

  /// Deletes an MCP server from the registry and every enabled live config.
  #[napi]
  pub fn delete_mcp_server(&self, server_id: String) -> Result<bool> {
    McpService::delete_server(self.state()?, &server_id).map_err(napi_error)
  }

  /// Enables or disables one MCP server for one supported application.
  #[napi]
  pub fn toggle_mcp_app(
    &self,
    server_id: String,
    #[napi(ts_arg_type = "import('./api.js').McpAppId")] app: String,
    enabled: bool,
  ) -> Result<()> {
    McpService::toggle_app(
      self.state()?,
      &server_id,
      parse_projection_app(&app, "MCP")?,
      enabled,
    )
    .map_err(napi_error)
  }

  /// Replaces the complete application matrix for an MCP server.
  #[napi]
  pub fn set_mcp_apps(
    &self,
    server_id: String,
    #[napi(ts_arg_type = "import('./api.js').McpApps")] apps: Value,
  ) -> Result<bool> {
    let apps: McpApps = parse_json(apps, "MCP application matrix")?;
    McpService::set_apps(self.state()?, &server_id, apps).map_err(napi_error)
  }

  /// Projects enabled MCP servers to every supported app, or to one app.
  #[napi]
  pub fn sync_mcp_to_live(
    &self,
    #[napi(ts_arg_type = "import('./api.js').McpAppId | null | undefined")] app: Option<String>,
  ) -> Result<()> {
    match app {
      Some(app) => {
        McpService::sync_enabled_for_app(self.state()?, &parse_projection_app(&app, "MCP")?)
          .map_err(napi_error)
      }
      None => McpService::sync_all_enabled(self.state()?).map_err(napi_error),
    }
  }

  /// Imports MCP servers from one supported app, or from every supported app.
  #[napi]
  pub fn import_mcp_from_live(
    &self,
    #[napi(ts_arg_type = "import('./api.js').McpAppId | null | undefined")] app: Option<String>,
  ) -> Result<u32> {
    let state = self.state()?;
    let count = match app.as_deref() {
      Some("claude") => McpService::import_from_claude(state),
      Some("codex") => McpService::import_from_codex(state),
      Some("gemini") => McpService::import_from_gemini(state),
      Some("opencode") => McpService::import_from_opencode(state),
      Some("hermes") => McpService::import_from_hermes(state),
      Some(other) => {
        return Err(Error::new(
          Status::InvalidArg,
          format!("Unsupported MCP app id: {other}"),
        ));
      }
      None => McpService::import_from_supported_apps(state),
    }
    .map_err(napi_error)?;
    count_to_u32(count, "Imported MCP server")
  }

  /// Returns applications that support managed Skills.
  #[napi(ts_return_type = "Array<import('./api.js').SkillAppId>")]
  pub fn supported_skill_apps(&self) -> Vec<String> {
    SkillService::supported_skill_apps()
      .map(|app| app.as_str().to_string())
      .collect()
  }

  /// Lists installed managed Skills.
  #[napi(ts_return_type = "Array<import('./api.js').InstalledSkill>")]
  pub fn list_skills(&self) -> Result<Vec<Value>> {
    self.state()?;
    SkillService::list_installed()
      .map_err(napi_error)?
      .into_iter()
      .map(serialize_json)
      .collect()
  }

  /// Installs a Skill from an upstream-supported spec and enables it for one app.
  #[napi(ts_return_type = "Promise<import('./api.js').InstalledSkill>")]
  pub async fn install_skill(
    &self,
    spec: String,
    #[napi(ts_arg_type = "import('./api.js').SkillAppId")] app: String,
  ) -> Result<Value> {
    self.state()?;
    let app = parse_projection_app(&app, "Skills")?;
    let service = SkillService::new().map_err(napi_error)?;
    let installed = service.install(&spec, &app).await.map_err(napi_error)?;
    serialize_json(installed)
  }

  /// Uninstalls a managed Skill from the SSOT and all application directories.
  #[napi]
  pub fn uninstall_skill(&self, directory_or_id: String) -> Result<()> {
    self.state()?;
    SkillService::uninstall(&directory_or_id).map_err(napi_error)
  }

  /// Enables or disables one installed Skill for one application.
  #[napi]
  pub fn toggle_skill_app(
    &self,
    directory_or_id: String,
    #[napi(ts_arg_type = "import('./api.js').SkillAppId")] app: String,
    enabled: bool,
  ) -> Result<()> {
    self.state()?;
    SkillService::toggle_app(
      &directory_or_id,
      &parse_projection_app(&app, "Skills")?,
      enabled,
    )
    .map_err(napi_error)
  }

  /// Replaces the complete application matrix for an installed Skill.
  #[napi]
  pub fn set_skill_apps(
    &self,
    directory_or_id: String,
    #[napi(ts_arg_type = "import('./api.js').SkillApps")] apps: Value,
  ) -> Result<bool> {
    self.state()?;
    let apps: SkillApps = parse_json(apps, "Skill application matrix")?;
    SkillService::set_apps(&directory_or_id, apps).map_err(napi_error)
  }

  /// Synchronizes enabled Skills to every supported app, or to one app.
  #[napi]
  pub fn sync_skills_to_live(
    &self,
    #[napi(ts_arg_type = "import('./api.js').SkillAppId | null | undefined")] app: Option<String>,
  ) -> Result<()> {
    self.state()?;
    let parsed = app
      .as_deref()
      .map(|app| parse_projection_app(app, "Skills"))
      .transpose()?;
    SkillService::sync_all_enabled(parsed.as_ref()).map_err(napi_error)
  }

  /// Scans application directories for Skills not yet managed by CC Switch.
  #[napi(ts_return_type = "Array<import('./api.js').UnmanagedSkill>")]
  pub fn scan_unmanaged_skills(&self) -> Result<Vec<Value>> {
    self.state()?;
    SkillService::scan_unmanaged()
      .map_err(napi_error)?
      .into_iter()
      .map(serialize_json)
      .collect()
  }

  /// Imports unmanaged Skills with an explicit per-Skill application matrix.
  #[napi(ts_return_type = "Array<import('./api.js').InstalledSkill>")]
  pub fn import_skills(
    &self,
    #[napi(ts_arg_type = "Array<import('./api.js').ImportSkillSelection>")] selections: Vec<Value>,
  ) -> Result<Vec<Value>> {
    self.state()?;
    let selections = selections
      .into_iter()
      .map(|value| parse_json::<ImportSkillSelection>(value, "Skill import selection"))
      .collect::<Result<Vec<_>>>()?;
    SkillService::import_from_apps(selections)
      .map_err(napi_error)?
      .into_iter()
      .map(serialize_json)
      .collect()
  }

  /// Lists configured Skill repositories.
  #[napi(ts_return_type = "Array<import('./api.js').SkillRepo>")]
  pub fn list_skill_repos(&self) -> Result<Vec<Value>> {
    self.state()?;
    SkillService::list_repos()
      .map_err(napi_error)?
      .into_iter()
      .map(serialize_json)
      .collect()
  }

  /// Adds or replaces a Skill repository.
  #[napi]
  pub fn upsert_skill_repo(
    &self,
    #[napi(ts_arg_type = "import('./api.js').SkillRepo")] repo: Value,
  ) -> Result<()> {
    self.state()?;
    let repo = parse_json(repo, "Skill repository")?;
    SkillService::upsert_repo(repo).map_err(napi_error)
  }

  /// Removes a Skill repository by owner/name.
  #[napi]
  pub fn remove_skill_repo(&self, owner: String, name: String) -> Result<()> {
    self.state()?;
    SkillService::remove_repo(&owner, &name).map_err(napi_error)
  }

  /// Returns the configured Skill deployment strategy.
  #[napi(ts_return_type = "import('./api.js').SkillSyncMethod")]
  pub fn skill_sync_method(&self) -> Result<String> {
    self.state()?;
    let method = SkillService::get_sync_method().map_err(napi_error)?;
    match serialize_json(method)? {
      Value::String(method) => Ok(method),
      _ => Err(Error::new(
        Status::GenericFailure,
        "Invalid Skill sync method returned by core".to_string(),
      )),
    }
  }

  /// Sets the Skill deployment strategy: auto, symlink, or copy.
  #[napi]
  pub fn set_skill_sync_method(
    &self,
    #[napi(ts_arg_type = "import('./api.js').SkillSyncMethod")] method: String,
  ) -> Result<()> {
    self.state()?;
    let mut index = SkillService::load_index().map_err(napi_error)?;
    index.sync_method = parse_json(Value::String(method), "Skill sync method")?;
    SkillService::save_index(&index).map_err(napi_error)
  }

  /// Discovers installable Skills from configured repositories.
  #[napi(ts_return_type = "Promise<Array<import('./api.js').DiscoverableSkill>>")]
  pub async fn discover_skills(&self, force_refresh: Option<bool>) -> Result<Vec<Value>> {
    self.state()?;
    let service = SkillService::new().map_err(napi_error)?;
    service
      .list_skills_cached(force_refresh.unwrap_or(false))
      .await
      .map_err(napi_error)?
      .into_iter()
      .map(serialize_json)
      .collect()
  }

  /// Searches the skills.sh catalog using the vendored upstream client.
  #[napi(ts_return_type = "Promise<import('./api.js').SkillSearchResult>")]
  pub async fn search_skills(
    &self,
    query: String,
    limit: Option<u32>,
    offset: Option<u32>,
  ) -> Result<Value> {
    self.state()?;
    let service = SkillService::new().map_err(napi_error)?;
    let result = service
      .search_skills_sh(
        &query,
        limit.unwrap_or(20) as usize,
        offset.unwrap_or(0) as usize,
      )
      .await
      .map_err(napi_error)?;
    serialize_json(result)
  }

  /// Checks installed repository-backed Skills for content updates.
  #[napi(ts_return_type = "Promise<import('./api.js').SkillUpdateCheckResult>")]
  pub async fn check_skill_updates(&self) -> Result<Value> {
    self.state()?;
    let service = SkillService::new().map_err(napi_error)?;
    let result = service.check_updates().await.map_err(napi_error)?;
    Ok(serde_json::json!({
      "updates": result
        .updates
        .into_iter()
        .map(|update| serde_json::json!({
          "id": update.id,
          "name": update.name,
          "directory": update.directory,
          "currentHash": update.current_hash,
          "remoteHash": update.remote_hash,
        }))
        .collect::<Vec<_>>(),
      "failures": result.failures,
    }))
  }

  /// Updates selected installed Skills and reports per-Skill failures.
  #[napi(ts_return_type = "Promise<import('./api.js').SkillUpdateBatchResult>")]
  pub async fn update_skills(&self, ids: Vec<String>) -> Result<Value> {
    self.state()?;
    let service = SkillService::new().map_err(napi_error)?;
    let result = service.update_skills(&ids).await;
    let updated = result
      .updated
      .into_iter()
      .map(serialize_json)
      .collect::<Result<Vec<_>>>()?;
    Ok(serde_json::json!({
      "updated": updated,
      "failures": result
        .failures
        .into_iter()
        .map(|failure| serde_json::json!({
          "id": failure.id,
          "error": failure.error,
        }))
        .collect::<Vec<_>>(),
    }))
  }
}

impl CcSwitch {
  fn state(&self) -> Result<&AppState> {
    self.state.as_ref().ok_or_else(|| {
      Error::new(
        Status::GenericFailure,
        "This CcSwitch instance has been closed".to_string(),
      )
    })
  }
}

impl Drop for CcSwitch {
  fn drop(&mut self) {
    self.state.take();
    INSTANCE_ACTIVE.store(false, Ordering::Release);
  }
}
