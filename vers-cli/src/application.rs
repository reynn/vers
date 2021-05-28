//! VersCli Abscissa Application

use crate::{commands::VersCliCmd, config::VersCliConfig};
use abscissa_core::{
    application::{self, AppCell},
    config, trace, Application, EntryPoint, FrameworkError, StandardPaths,
};

/// Application state
pub static APPLICATION: AppCell<VersCliApp> = AppCell::new();

/// Obtain a read-only (multi-reader) lock on the application state.
///
/// Panics if the application state has not been initialized.
pub fn app_reader() -> application::lock::Reader<VersCliApp> {
    APPLICATION.read()
}

/// Obtain an exclusive mutable lock on the application state.
pub fn app_writer() -> application::lock::Writer<VersCliApp> {
    APPLICATION.write()
}

/// Obtain a read-only (multi-reader) lock on the application configuration.
///
/// Panics if the application configuration has not been loaded.
pub fn app_config() -> config::Reader<VersCliApp> {
    config::Reader::new(&APPLICATION)
}

/// VersCli Application
#[derive(Debug)]
pub struct VersCliApp {
    /// Application configuration.
    config: Option<VersCliConfig>,

    /// Application state.
    state: application::State<Self>,
}

/// Initialize a new application instance.
///
/// By default no configuration is loaded, and the framework state is
/// initialized to a default, empty state (no components, threads, etc).
impl Default for VersCliApp {
    fn default() -> Self {
        Self {
            config: None,
            state: application::State::default(),
        }
    }
}

impl Application for VersCliApp {
    /// Entrypoint command for this application.
    type Cmd = EntryPoint<VersCliCmd>;

    /// Application configuration.
    type Cfg = VersCliConfig;

    /// Paths to resources within the application.
    type Paths = StandardPaths;

    /// Accessor for application configuration.
    fn config(&self) -> &VersCliConfig {
        self.config.as_ref().expect("config not loaded")
    }

    /// Borrow the application state immutably.
    fn state(&self) -> &application::State<Self> {
        &self.state
    }

    /// Borrow the application state mutably.
    fn state_mut(&mut self) -> &mut application::State<Self> {
        &mut self.state
    }

    /// Register all components used by this application.
    ///
    /// If you would like to add additional components to your application
    /// beyond the default ones provided by the framework, this is the place
    /// to do so.
    fn register_components(&mut self, command: &Self::Cmd) -> Result<(), FrameworkError> {
        let components = self.framework_components(command)?;
        self.state.components.register(components)
    }

    /// Post-configuration lifecycle callback.
    ///
    /// Called regardless of whether config is loaded to indicate this is the
    /// time in app lifecycle when configuration would be loaded if
    /// possible.
    fn after_config(&mut self, config: Self::Cfg) -> Result<(), FrameworkError> {
        // Configure components
        self.state.components.after_config(&config)?;
        self.config = Some(config);
        Ok(())
    }

    /// Get tracing configuration from command-line options
    fn tracing_config(&self, command: &EntryPoint<VersCliCmd>) -> trace::Config {
        if command.verbose {
            trace::Config::verbose()
        } else {
            trace::Config::default()
        }
    }
}
