//! Config port — where the `Config` value comes from.
//! Impl: `kuda-config-toml` (file); env/remote sources = new adapters.

/// A config source — returns the loaded `Config`.
pub trait ConfigSource: Send + Sync {
    /// Load + validate the config; Err carries a readable message.
    fn load(&self) -> Result<kuda_core::Config, String>;
}
