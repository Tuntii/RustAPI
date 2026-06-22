//! CLI commands

mod add;
mod bench;
mod client;
mod deploy;
mod docs;
mod doctor;
mod generate;
#[cfg(feature = "cloud")]
mod login;
mod logout;
mod migrate;
mod new;
mod observability;
mod run;
mod watch;
mod whoami;

pub use add::{add, AddArgs};
pub use bench::{bench, BenchArgs};
pub use client::{client, ClientArgs};
pub use deploy::{deploy, DeployArgs};
pub use docs::open_docs;
pub use doctor::{doctor, DoctorArgs};
pub use generate::{generate, GenerateArgs};
#[cfg(feature = "cloud")]
pub use login::{login, LoginArgs};
pub use logout::{logout, LogoutArgs};
pub use migrate::{migrate, MigrateArgs};
pub use new::{new_project, NewArgs};
pub use observability::{observability, ObservabilityArgs};
pub use run::{run_dev, RunArgs};
pub use watch::{watch, WatchArgs};
pub use whoami::{whoami, WhoamiArgs};

#[cfg(feature = "replay")]
mod replay;
#[cfg(feature = "replay")]
pub use replay::{replay, ReplayArgs};

#[cfg(feature = "mcp")]
mod mcp;
#[cfg(feature = "mcp")]
pub use mcp::{mcp_generate, McpGenerateArgs};
