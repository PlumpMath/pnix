//! V02: Runtime Plugin System
//!
//! Provides a plugin architecture for extending Pnix runtimes with
//! custom backends, capabilities, and execution strategies.
//!
//! # Overview
//!
//! The plugin system consists of:
//! - `RuntimePlugin`: Trait that plugins must implement
//! - `PluginInfo`: Metadata about a plugin (name, version, capabilities)
//! - `PluginRegistry`: Central registry for managing plugins
//!
//! # Example
//!
//! ```
//! use pnix_runtime_api::plugin::{CapabilityPolicy, PluginInfo, PluginRegistry, RuntimePlugin};
//! use pnix_runtime_api::{RuntimeCapabilities, RuntimeCapability};
//!
//! struct MyPlugin;
//!
//! impl RuntimePlugin for MyPlugin {
//!     fn info(&self) -> PluginInfo {
//!         PluginInfo::new("my-plugin", "1.0.0")
//!             .with_description("My custom plugin")
//!             .with_capability(RuntimeCapability::Pure)
//!     }
//!
//!     fn initialize(&mut self) -> Result<(), String> {
//!         Ok(())
//!     }
//! }
//!
//! let mut registry = PluginRegistry::new()
//!     .with_capability_policy(CapabilityPolicy::allow_all());
//! registry.register(Box::new(MyPlugin));
//! ```

use crate::{RuntimeCapabilities, RuntimeCapability};
use std::collections::HashMap;
use std::sync::RwLock;

/// Plugin metadata
#[derive(Debug, Clone)]
pub struct PluginInfo {
  /// Unique plugin identifier
  pub name: String,
  /// Plugin version (semver)
  pub version: String,
  /// Human-readable description
  pub description: Option<String>,
  /// Capabilities provided by this plugin
  pub capabilities: RuntimeCapabilities,
  /// Plugin author
  pub author: Option<String>,
  /// Plugin homepage/repository
  pub homepage: Option<String>,
}

impl PluginInfo {
  /// Create new plugin info with name and version
  pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
    Self {
      name: name.into(),
      version: version.into(),
      description: None,
      capabilities: RuntimeCapabilities::new(),
      author: None,
      homepage: None,
    }
  }

  /// Add description
  pub fn with_description(mut self, desc: impl Into<String>) -> Self {
    self.description = Some(desc.into());
    self
  }

  /// Add a capability
  pub fn with_capability(mut self, cap: RuntimeCapability) -> Self {
    self.capabilities = self.capabilities.with_capability(cap);
    self
  }

  /// Add an engine
  pub fn with_engine(mut self, engine: impl Into<String>) -> Self {
    self.capabilities = self.capabilities.with_engine(engine);
    self
  }

  /// Add an option
  pub fn with_option(mut self, option: impl Into<String>) -> Self {
    self.capabilities = self.capabilities.with_option(option);
    self
  }

  /// Set author
  pub fn with_author(mut self, author: impl Into<String>) -> Self {
    self.author = Some(author.into());
    self
  }

  /// Set homepage
  pub fn with_homepage(mut self, homepage: impl Into<String>) -> Self {
    self.homepage = Some(homepage.into());
    self
  }
}

/// Plugin lifecycle state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginState {
  /// Plugin is registered but not initialized
  Registered,
  /// Plugin is initialized and ready
  Ready,
  /// Plugin encountered an error
  Error,
  /// Plugin is disabled
  Disabled,
}

impl std::fmt::Display for PluginState {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Registered => write!(f, "registered"),
      Self::Ready => write!(f, "ready"),
      Self::Error => write!(f, "error"),
      Self::Disabled => write!(f, "disabled"),
    }
  }
}

/// Runtime plugin trait
///
/// Plugins must implement this trait to be registered with the runtime.
pub trait RuntimePlugin: Send + Sync {
  /// Get plugin metadata
  fn info(&self) -> PluginInfo;

  /// Initialize the plugin
  ///
  /// Called once when the plugin is first loaded. Return an error
  /// string if initialization fails.
  fn initialize(&mut self) -> Result<(), String>;

  /// Shutdown the plugin
  ///
  /// Called when the plugin is being unloaded. Perform cleanup here.
  fn shutdown(&mut self) -> Result<(), String> {
    Ok(())
  }

  /// Check if the plugin is healthy
  fn health_check(&self) -> bool {
    true
  }
}

/// Entry in the plugin registry
struct PluginEntry {
  plugin: Box<dyn RuntimePlugin>,
  info: PluginInfo,
  state: PluginState,
  error_message: Option<String>,
}

/// Capability policy for plugin registration and aggregation.
#[derive(Debug, Clone)]
pub enum CapabilityPolicy {
  /// Accept all plugin-declared capabilities (unrestricted).
  AllowAll,
  /// Allow only explicitly listed capabilities/engines/options.
  AllowList(RuntimeCapabilities),
}

impl CapabilityPolicy {
  pub fn allow_all() -> Self {
    Self::AllowAll
  }

  pub fn allow_list(allowed: RuntimeCapabilities) -> Self {
    Self::AllowList(allowed)
  }

  fn validate(&self, requested: &RuntimeCapabilities) -> Result<(), String> {
    match self {
      Self::AllowAll => Ok(()),
      Self::AllowList(allowed) => {
        let mut denied = Vec::new();
        let mut denied_engines = Vec::new();
        let mut denied_options = Vec::new();

        for cap in &requested.capabilities {
          if !allowed.supports(cap) {
            denied.push(cap.to_string());
          }
        }
        for engine in &requested.engines {
          if !allowed.supports_engine(engine) {
            denied_engines.push(engine.clone());
          }
        }
        for option in &requested.options {
          if !allowed.supports_option(option) {
            denied_options.push(option.clone());
          }
        }

        if denied.is_empty() && denied_engines.is_empty() && denied_options.is_empty() {
          return Ok(());
        }

        let mut parts = Vec::new();
        if !denied.is_empty() {
          parts.push(format!("capabilities: {}", denied.join(", ")));
        }
        if !denied_engines.is_empty() {
          parts.push(format!("engines: {}", denied_engines.join(", ")));
        }
        if !denied_options.is_empty() {
          parts.push(format!("options: {}", denied_options.join(", ")));
        }

        Err(format!(
          "plugin capability policy rejected: {}",
          parts.join("; ")
        ))
      }
    }
  }

  fn filter(&self, requested: &RuntimeCapabilities) -> RuntimeCapabilities {
    match self {
      Self::AllowAll => requested.clone(),
      Self::AllowList(allowed) => {
        let mut filtered = RuntimeCapabilities::new();
        for cap in &requested.capabilities {
          if allowed.supports(cap) {
            filtered = filtered.with_capability(*cap);
          }
        }
        for engine in &requested.engines {
          if allowed.supports_engine(engine) {
            filtered = filtered.with_engine(engine.clone());
          }
        }
        for option in &requested.options {
          if allowed.supports_option(option) {
            filtered = filtered.with_option(option.clone());
          }
        }
        filtered
      }
    }
  }
}

impl Default for CapabilityPolicy {
  fn default() -> Self {
    Self::AllowList(RuntimeCapabilities::new())
  }
}

fn validate_plugin_name(name: &str) -> Result<(), String> {
  if name.is_empty() {
    return Err("Plugin name must not be empty".to_string());
  }
  if name == "." || name == ".." {
    return Err("Plugin name must not be '.' or '..'".to_string());
  }
  if name.contains('/') || name.contains('\\') {
    return Err("Plugin name must not contain path separators".to_string());
  }
  if name.chars().any(|c| c.is_control()) {
    return Err("Plugin name must not contain control characters".to_string());
  }
  Ok(())
}

/// Central registry for managing plugins
pub struct PluginRegistry {
  plugins: RwLock<HashMap<String, PluginEntry>>,
  capability_policy: CapabilityPolicy,
}

impl PluginRegistry {
  /// Create a new empty registry (deny-by-default capability policy).
  pub fn new() -> Self {
    Self {
      plugins: RwLock::new(HashMap::new()),
      capability_policy: CapabilityPolicy::default(),
    }
  }

  /// Create a new registry that accepts all plugin-declared capabilities.
  pub fn new_unrestricted() -> Self {
    Self {
      plugins: RwLock::new(HashMap::new()),
      capability_policy: CapabilityPolicy::allow_all(),
    }
  }

  /// Configure capability policy for plugin registration and aggregation.
  pub fn with_capability_policy(mut self, policy: CapabilityPolicy) -> Self {
    self.capability_policy = policy;
    self
  }

  /// Update capability policy for plugin registration and aggregation.
  pub fn set_capability_policy(&mut self, policy: CapabilityPolicy) {
    self.capability_policy = policy;
  }

  /// Register a plugin (subject to the capability policy).
  ///
  /// Returns the plugin name if successful, or an error if a plugin
  /// with the same name is already registered.
  pub fn register(&mut self, plugin: Box<dyn RuntimePlugin>) -> Result<String, String> {
    let info = plugin.info();
    let name = info.name.clone();
    validate_plugin_name(&name)?;
    self.capability_policy.validate(&info.capabilities)?;

    let mut plugins = self.plugins.write().unwrap_or_else(|e| e.into_inner());
    if plugins.contains_key(&name) {
      return Err(format!("Plugin '{}' is already registered", name));
    }

    plugins.insert(
      name.clone(),
      PluginEntry {
        plugin,
        info,
        state: PluginState::Registered,
        error_message: None,
      },
    );

    Ok(name)
  }

  /// Initialize a registered plugin
  pub fn initialize(&mut self, name: &str) -> Result<(), String> {
    let mut plugins = self.plugins.write().unwrap_or_else(|e| e.into_inner());
    let entry = plugins
      .get_mut(name)
      .ok_or_else(|| format!("Plugin '{}' not found", name))?;

    // 이미 초기화된 플러그인은 재초기화하지 않음
    if entry.state == PluginState::Ready {
      return Ok(());
    }

    match entry.plugin.initialize() {
      Ok(()) => {
        entry.state = PluginState::Ready;
        entry.error_message = None;
        Ok(())
      }
      Err(e) => {
        entry.state = PluginState::Error;
        entry.error_message = Some(e.clone());
        Err(e)
      }
    }
  }

  /// Initialize all registered plugins
  pub fn initialize_all(&mut self) -> Vec<(String, Result<(), String>)> {
    let mut names: Vec<_> = {
      let plugins = self.plugins.read().unwrap_or_else(|e| e.into_inner());
      plugins.keys().cloned().collect()
    };
    // 결정론적 순서 보장을 위해 정렬
    names.sort();
    names
      .into_iter()
      .map(|name| {
        let result = self.initialize(&name);
        (name, result)
      })
      .collect()
  }

  /// Get plugin info by name
  pub fn get_info(&self, name: &str) -> Option<PluginInfo> {
    let plugins = self.plugins.read().unwrap_or_else(|e| e.into_inner());
    plugins.get(name).map(|e| e.info.clone())
  }

  /// Get plugin state by name
  pub fn get_state(&self, name: &str) -> Option<PluginState> {
    let plugins = self.plugins.read().unwrap_or_else(|e| e.into_inner());
    plugins.get(name).map(|e| e.state)
  }

  /// List all registered plugin names
  pub fn list(&self) -> Vec<String> {
    let plugins = self.plugins.read().unwrap_or_else(|e| e.into_inner());
    plugins.keys().cloned().collect()
  }

  /// List all plugins with their states
  pub fn list_with_state(&self) -> Vec<(String, PluginState)> {
    let plugins = self.plugins.read().unwrap_or_else(|e| e.into_inner());
    plugins
      .iter()
      .map(|(name, entry)| (name.clone(), entry.state))
      .collect()
  }

  /// Get combined capabilities from all ready plugins
  pub fn combined_capabilities(&self) -> RuntimeCapabilities {
    let mut combined = RuntimeCapabilities::new();

    let plugins = self.plugins.read().unwrap_or_else(|e| e.into_inner());
    for entry in plugins.values() {
      if entry.state == PluginState::Ready {
        let caps = self.capability_policy.filter(&entry.info.capabilities);
        for cap in caps.capabilities {
          combined = combined.with_capability(cap);
        }
        for engine in caps.engines {
          combined = combined.with_engine(engine);
        }
        for option in caps.options {
          combined = combined.with_option(option);
        }
      }
    }

    combined
  }

  /// Unregister a plugin
  pub fn unregister(&mut self, name: &str) -> Result<(), String> {
    let mut plugins = self.plugins.write().unwrap_or_else(|e| e.into_inner());
    if let Some(mut entry) = plugins.remove(name) {
      // 초기화된 플러그인만 shutdown 호출
      if entry.state == PluginState::Ready {
        entry.plugin.shutdown()?;
      }
      Ok(())
    } else {
      Err(format!("Plugin '{}' not found", name))
    }
  }

  /// Shutdown all plugins and clear the registry
  pub fn shutdown_all(&mut self) -> Vec<(String, Result<(), String>)> {
    let names: Vec<_> = {
      let plugins = self.plugins.read().unwrap_or_else(|e| e.into_inner());
      plugins.keys().cloned().collect()
    };
    names
      .into_iter()
      .map(|name| {
        let result = self.unregister(&name);
        (name, result)
      })
      .collect()
  }

  /// Get the number of registered plugins
  pub fn len(&self) -> usize {
    let plugins = self.plugins.read().unwrap_or_else(|e| e.into_inner());
    plugins.len()
  }

  /// Check if the registry is empty
  pub fn is_empty(&self) -> bool {
    let plugins = self.plugins.read().unwrap_or_else(|e| e.into_inner());
    plugins.is_empty()
  }
}

impl Default for PluginRegistry {
  fn default() -> Self {
    Self::new()
  }
}

// ========================================
// V02: Dynamic Plugin Loading (feature-gated)
// ========================================

#[cfg(feature = "dynamic-plugins")]
pub mod dynamic {
  //! Dynamic plugin loading support.
  //!
  //! This module provides functionality to load plugins from shared libraries
  //! (.so on Linux, .dylib on macOS, .dll on Windows).
  //!
  //! # Safety
  //!
  //! Loading dynamic libraries is inherently unsafe. Plugins must be compiled
  //! with the same Rust version and ABI as the host application.
  //!
  //! # Status
  //!
  //! **Experimental**: This is a POC implementation.

  use super::*;
  use std::path::Path;

  /// Error type for dynamic plugin loading
  #[derive(Debug)]
  pub enum DynamicPluginError {
    /// Library loading failed
    LoadError(String),
    /// Symbol not found
    SymbolNotFound(String),
    /// Plugin creation failed
    CreationFailed(String),
    /// ABI mismatch
    AbiMismatch { expected: String, found: String },
  }

  impl std::fmt::Display for DynamicPluginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      match self {
        Self::LoadError(e) => write!(f, "failed to load library: {}", e),
        Self::SymbolNotFound(s) => write!(f, "symbol not found: {}", s),
        Self::CreationFailed(e) => write!(f, "plugin creation failed: {}", e),
        Self::AbiMismatch { expected, found } => {
          write!(f, "ABI mismatch: expected {}, found {}", expected, found)
        }
      }
    }
  }

  impl std::error::Error for DynamicPluginError {}

  /// ABI version for plugin compatibility
  pub const PLUGIN_ABI_VERSION: &str = "pnix-plugin-abi-v1";

  /// Opaque string payload passed over the C ABI.
  #[repr(C)]
  pub struct PluginApiString {
    pub ptr: *const std::ffi::c_char,
    pub len: usize,
    pub free: unsafe extern "C" fn(*const std::ffi::c_char, usize),
  }

  impl PluginApiString {
    pub fn empty() -> Self {
      Self {
        ptr: std::ptr::null(),
        len: 0,
        free: plugin_api_string_noop_free,
      }
    }
  }

  /// ABI result type for plugin calls.
  #[repr(C)]
  pub struct PluginApiResult {
    pub ok: bool,
    pub message: PluginApiString,
  }

  /// Stable C ABI surface for dynamic plugins.
  #[repr(C)]
  pub struct PluginApiV1 {
    pub abi_version: *const std::ffi::c_char,
    pub drop: unsafe extern "C" fn(*mut PluginApiV1),
    pub info_json: unsafe extern "C" fn(*mut PluginApiV1) -> PluginApiString,
    pub initialize: unsafe extern "C" fn(*mut PluginApiV1) -> PluginApiResult,
    pub shutdown: unsafe extern "C" fn(*mut PluginApiV1) -> PluginApiResult,
    pub health_check: unsafe extern "C" fn(*mut PluginApiV1) -> bool,
  }

  unsafe extern "C" fn plugin_api_string_noop_free(_ptr: *const std::ffi::c_char, _len: usize) {}

  /// Type signature for the plugin creation function.
  ///
  /// Plugins must export a function with this signature named `_pnix_plugin_create`.
  pub type PluginCreateFn = unsafe extern "C" fn() -> *mut PluginApiV1;

  /// Type signature for the ABI version function
  ///
  /// Plugins must export a function with this signature named `_pnix_plugin_abi_version`
  pub type PluginAbiVersionFn = unsafe extern "C" fn() -> *const std::ffi::c_char;

  /// Dynamic plugin loader
  ///
  /// # Example
  ///
  /// ```ignore
  /// use pnix_runtime_api::plugin::dynamic::DynamicPluginLoader;
  ///
  /// let mut loader = DynamicPluginLoader::new();
  /// let plugin = loader.load("./libmy_plugin.so")?;
  /// ```
  pub struct DynamicPluginLoader {
    /// Loaded libraries (kept alive to prevent unloading)
    #[allow(dead_code)]
    libraries: Vec<libloading::Library>,
  }

  impl DynamicPluginLoader {
    /// Validate plugin path to prevent path traversal attacks
    fn validate_plugin_path(path: &Path) -> Result<(), String> {
      // 절대 경로 거부
      if path.is_absolute() {
        return Err("absolute paths are not allowed".to_string());
      }

      // 경로 컴포넌트 검증
      for component in path.components() {
        match component {
          std::path::Component::ParentDir => {
            return Err("parent directory (..) is not allowed".to_string());
          }
          std::path::Component::RootDir => {
            return Err("root directory is not allowed".to_string());
          }
          std::path::Component::Prefix(_) => {
            return Err("path prefix is not allowed".to_string());
          }
          std::path::Component::CurDir | std::path::Component::Normal(_) => {
            // 허용
          }
        }
      }

      // 빈 경로 거부
      if path.as_os_str().is_empty() {
        return Err("empty path is not allowed".to_string());
      }

      Ok(())
    }

    /// Create a new plugin loader
    pub fn new() -> Self {
      Self {
        libraries: Vec::new(),
      }
    }

    /// Load a plugin from a shared library
    ///
    /// # Safety
    ///
    /// This function loads and executes code from an external library.
    /// The library must be trusted and compiled with compatible ABI.
    ///
    /// # POC Status
    ///
    /// Currently returns unimplemented error. Full implementation would:
    /// 1. Validate plugin path (prevent path traversal)
    /// 2. Load the library
    /// 3. Check ABI version
    /// 4. Call the creation function
    /// 5. Return the plugin
    pub fn load(
      &mut self,
      path: impl AsRef<Path>,
    ) -> Result<Box<dyn RuntimePlugin>, DynamicPluginError> {
      let path = path.as_ref();

      // 경로 순회 방지 검증
      if let Err(e) = Self::validate_plugin_path(path) {
        return Err(DynamicPluginError::LoadError(format!(
          "Invalid plugin path: {}",
          e
        )));
      }

      // POC: Return unimplemented error
      // Full implementation would use libloading to load the shared library
      Err(DynamicPluginError::LoadError(
        "[PLUGIN-001] experimental dynamic plugin loading path is not implemented; \
         loading is disabled for safety (POC V02)."
          .to_string(),
      ))
    }

    /// Get the number of loaded libraries
    pub fn loaded_count(&self) -> usize {
      self.libraries.len()
    }
  }

  impl Default for DynamicPluginLoader {
    fn default() -> Self {
      Self::new()
    }
  }

  #[cfg(test)]
  mod tests {
    use super::*;

    #[test]
    fn test_dynamic_loader_creation() {
      let loader = DynamicPluginLoader::new();
      assert_eq!(loader.loaded_count(), 0);
    }

    #[test]
    fn test_dynamic_load_returns_unimplemented() {
      let mut loader = DynamicPluginLoader::new();
      let result = loader.load("plugins/nonexistent.plugin");
      assert!(result.is_err());

      match result {
        Err(DynamicPluginError::LoadError(msg)) => {
          assert!(msg.contains("[PLUGIN-001]"));
          assert!(msg.contains("not implemented"));
        }
        _ => panic!("Expected LoadError"),
      }
    }

    #[test]
    fn test_abi_version_constant() {
      assert_eq!(PLUGIN_ABI_VERSION, "pnix-plugin-abi-v1");
    }

    #[test]
    fn test_plugin_api_string_empty() {
      let empty = PluginApiString::empty();
      assert!(empty.ptr.is_null());
      assert_eq!(empty.len, 0);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::sync::atomic::{AtomicUsize, Ordering};

  struct TestPlugin {
    name: String,
    init_should_fail: bool,
  }

  impl TestPlugin {
    fn new(name: &str) -> Self {
      Self {
        name: name.to_string(),
        init_should_fail: false,
      }
    }

    fn failing(name: &str) -> Self {
      Self {
        name: name.to_string(),
        init_should_fail: true,
      }
    }
  }

  impl RuntimePlugin for TestPlugin {
    fn info(&self) -> PluginInfo {
      PluginInfo::new(&self.name, "1.0.0")
        .with_description("Test plugin")
        .with_capability(RuntimeCapability::Pure)
        .with_engine("test")
    }

    fn initialize(&mut self) -> Result<(), String> {
      if self.init_should_fail {
        Err("Initialization failed".to_string())
      } else {
        Ok(())
      }
    }
  }

  struct FlippingPlugin {
    calls: AtomicUsize,
  }

  impl RuntimePlugin for FlippingPlugin {
    fn info(&self) -> PluginInfo {
      let call = self.calls.fetch_add(1, Ordering::SeqCst);
      if call == 0 {
        PluginInfo::new("flip", "1.0.0").with_capability(RuntimeCapability::Pure)
      } else {
        PluginInfo::new("flip", "1.0.0").with_capability(RuntimeCapability::Network)
      }
    }

    fn initialize(&mut self) -> Result<(), String> {
      Ok(())
    }
  }

  #[test]
  fn test_plugin_info_builder() {
    let info = PluginInfo::new("test", "1.0.0")
      .with_description("A test plugin")
      .with_capability(RuntimeCapability::Pure)
      .with_capability(RuntimeCapability::Math)
      .with_engine("test-engine")
      .with_option("deterministic")
      .with_author("Test Author")
      .with_homepage("https://example.com");

    assert_eq!(info.name, "test");
    assert_eq!(info.version, "1.0.0");
    assert_eq!(info.description, Some("A test plugin".to_string()));
    assert!(info.capabilities.supports(&RuntimeCapability::Pure));
    assert!(info.capabilities.supports(&RuntimeCapability::Math));
    assert!(info.capabilities.supports_engine("test-engine"));
    assert!(info.capabilities.supports_option("deterministic"));
    assert_eq!(info.author, Some("Test Author".to_string()));
  }

  #[test]
  fn test_plugin_registry_basic() {
    let mut registry = PluginRegistry::new_unrestricted();

    // Register a plugin
    let result = registry.register(Box::new(TestPlugin::new("plugin-a")));
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "plugin-a");

    // Check it's registered
    assert_eq!(registry.len(), 1);
    assert!(registry.get_info("plugin-a").is_some());
    assert_eq!(
      registry.get_state("plugin-a"),
      Some(PluginState::Registered)
    );
  }

  #[test]
  fn test_plugin_info_is_frozen_on_register() {
    let mut registry = PluginRegistry::new_unrestricted();
    let name = registry
      .register(Box::new(FlippingPlugin {
        calls: AtomicUsize::new(0),
      }))
      .unwrap();

    let info = registry.get_info(&name).expect("info");
    assert!(info.capabilities.supports(&RuntimeCapability::Pure));
    assert!(!info.capabilities.supports(&RuntimeCapability::Network));

    registry.initialize(&name).unwrap();
    let caps = registry.combined_capabilities();
    assert!(caps.supports(&RuntimeCapability::Pure));
    assert!(!caps.supports(&RuntimeCapability::Network));
  }

  #[test]
  fn test_plugin_registry_rejects_invalid_name() {
    let mut registry = PluginRegistry::new_unrestricted();
    let bad_names = ["", ".", "..", "bad/name", "bad\\name", "bad\0name"];

    for name in bad_names {
      let result = registry.register(Box::new(TestPlugin::new(name)));
      assert!(result.is_err(), "expected error for name: {:?}", name);
    }
  }

  #[test]
  fn test_plugin_registry_duplicate() {
    let mut registry = PluginRegistry::new_unrestricted();

    registry
      .register(Box::new(TestPlugin::new("plugin-a")))
      .unwrap();

    // Try to register again
    let result = registry.register(Box::new(TestPlugin::new("plugin-a")));
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("already registered"));
  }

  #[test]
  fn test_plugin_initialization() {
    let mut registry = PluginRegistry::new_unrestricted();
    registry
      .register(Box::new(TestPlugin::new("plugin-a")))
      .unwrap();

    // Initialize
    let result = registry.initialize("plugin-a");
    assert!(result.is_ok());
    assert_eq!(registry.get_state("plugin-a"), Some(PluginState::Ready));
  }

  #[test]
  fn test_plugin_initialization_failure() {
    let mut registry = PluginRegistry::new_unrestricted();
    registry
      .register(Box::new(TestPlugin::failing("plugin-fail")))
      .unwrap();

    // Initialize should fail
    let result = registry.initialize("plugin-fail");
    assert!(result.is_err());
    assert_eq!(registry.get_state("plugin-fail"), Some(PluginState::Error));
  }

  #[test]
  fn test_combined_capabilities() {
    let mut registry = PluginRegistry::new_unrestricted();

    // Register and initialize two plugins
    registry
      .register(Box::new(TestPlugin::new("plugin-a")))
      .unwrap();
    registry
      .register(Box::new(TestPlugin::new("plugin-b")))
      .unwrap();
    registry.initialize_all();

    // Check combined capabilities
    let caps = registry.combined_capabilities();
    assert!(caps.supports(&RuntimeCapability::Pure));
    assert!(caps.supports_engine("test"));
  }

  #[test]
  fn test_plugin_unregister() {
    let mut registry = PluginRegistry::new_unrestricted();
    registry
      .register(Box::new(TestPlugin::new("plugin-a")))
      .unwrap();
    registry.initialize("plugin-a").unwrap();

    // Unregister
    let result = registry.unregister("plugin-a");
    assert!(result.is_ok());
    assert_eq!(registry.len(), 0);
  }

  #[test]
  fn test_plugin_list() {
    let mut registry = PluginRegistry::new_unrestricted();
    registry
      .register(Box::new(TestPlugin::new("plugin-a")))
      .unwrap();
    registry
      .register(Box::new(TestPlugin::new("plugin-b")))
      .unwrap();
    registry.initialize("plugin-a").unwrap();

    let list = registry.list_with_state();
    assert_eq!(list.len(), 2);

    // Find each plugin
    let a = list.iter().find(|(n, _)| n == "plugin-a");
    let b = list.iter().find(|(n, _)| n == "plugin-b");

    assert!(matches!(a, Some((_, PluginState::Ready))));
    assert!(matches!(b, Some((_, PluginState::Registered))));
  }

  #[test]
  fn test_plugin_policy_rejects_capabilities() {
    let mut registry = PluginRegistry::new();

    let result = registry.register(Box::new(TestPlugin::new("plugin-a")));
    assert!(result.is_err());
    assert!(result
      .unwrap_err()
      .contains("plugin capability policy rejected"));
  }
}
