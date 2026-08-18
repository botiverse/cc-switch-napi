#![deny(clippy::all)]

use std::{
  str::FromStr,
  sync::atomic::{AtomicBool, Ordering},
};

use cc_switch_lib::{AppState, AppType, Provider, ProviderService};
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

fn parse_provider(value: Value) -> Result<Provider> {
  serde_json::from_value(value).map_err(|error| {
    Error::new(
      Status::InvalidArg,
      format!("Invalid provider object: {error}"),
    )
  })
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
  state: AppState,
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
      Ok(state) => Ok(Self { state }),
      Err(error) => {
        INSTANCE_ACTIVE.store(false, Ordering::Release);
        Err(napi_error(error))
      }
    }
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
    ProviderService::list(&self.state, app)
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
    ProviderService::current(&self.state, app)
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
    ProviderService::add(&self.state, parse_app(&app)?, parse_provider(provider)?)
      .map_err(napi_error)
  }

  /// Replaces an existing provider by id.
  #[napi]
  pub fn update_provider(
    &self,
    #[napi(ts_arg_type = "import('./api.js').AppId")] app: String,
    #[napi(ts_arg_type = "import('./api.js').Provider")] provider: Value,
  ) -> Result<bool> {
    ProviderService::update(&self.state, parse_app(&app)?, parse_provider(provider)?)
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
    ProviderService::duplicate(&self.state, parse_app(&app)?, &source_id, provider_override)
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
    ProviderService::switch(&self.state, parse_app(&app)?, &provider_id).map_err(napi_error)
  }

  /// Deletes a provider. CC Switch refuses to delete an active provider.
  #[napi]
  pub fn delete_provider(
    &self,
    #[napi(ts_arg_type = "import('./api.js').AppId")] app: String,
    provider_id: String,
  ) -> Result<()> {
    ProviderService::delete(&self.state, parse_app(&app)?, &provider_id).map_err(napi_error)
  }

  /// Imports providers from the application's current live configuration.
  #[napi]
  pub fn import_live_config(
    &self,
    #[napi(ts_arg_type = "import('./api.js').AppId")] app: String,
  ) -> Result<u32> {
    ProviderService::import_live_config(&self.state, parse_app(&app)?)
      .and_then(|count| {
        u32::try_from(count).map_err(|_| {
          cc_switch_lib::AppError::Message("Imported provider count exceeds u32".to_string())
        })
      })
      .map_err(napi_error)
  }

  /// Removes a provider from an additive live config without deleting it.
  #[napi]
  pub fn remove_from_live_config(
    &self,
    #[napi(ts_arg_type = "import('./api.js').AppId")] app: String,
    provider_id: String,
  ) -> Result<()> {
    ProviderService::remove_from_live_config(&self.state, parse_app(&app)?, &provider_id)
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
      &self.state,
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

  /// Writes all currently selected providers back to their live configs.
  #[napi]
  pub fn sync_current_to_live(&self) -> Result<()> {
    ProviderService::sync_current_to_live(&self.state).map_err(napi_error)
  }
}

impl Drop for CcSwitch {
  fn drop(&mut self) {
    INSTANCE_ACTIVE.store(false, Ordering::Release);
  }
}
