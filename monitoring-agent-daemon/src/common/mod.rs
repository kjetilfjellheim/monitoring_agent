/**
 * Common module. Contains the common types used by the monitoring agent daemon.
 * 
 * `applicationerror`: The application error. Used to represent an error in the application.
 * `monitorstatus`: The monitor status. Used to represent the status of a monitor in the services.
 * `configuration`: The configuration. Used to represent the configuration of the monitoring agent daemon.
 * `args`: The application arguments. Used to represent the arguments passed to the application.
 */
mod applicationerror;
mod monitorstatus;
pub mod configuration;
pub mod args;

pub use crate::common::applicationerror::ApplicationError;
pub use crate::common::monitorstatus::{MonitorStatus, Status};
pub use crate::common::configuration::{Monitor, MonitorType, HttpMethod, DatabaseConfig};
pub use crate::common::args::ApplicationArguments;
