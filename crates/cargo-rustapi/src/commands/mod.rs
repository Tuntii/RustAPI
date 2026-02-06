//! CLI commands

mod add;
mod client;
mod deploy;
mod docs;
mod doctor;
mod generate;
mod migrate;
mod new;
mod run;
mod watch;

pub use add::{add, AddArgs};
pub use client::{client, ClientArgs};
pub use deploy::{deploy, DeployArgs};
pub use docs::open_docs;
pub use doctor::{doctor, DoctorArgs};
pub use generate::{generate, GenerateArgs};
pub use migrate::{migrate, MigrateArgs};
pub use new::{new_project, NewArgs};
pub use run::{run_dev, RunArgs};
pub use watch::{watch, WatchArgs};

#[cfg(feature = "replay")]
mod replay;
#[cfg(feature = "replay")]
pub use replay::{replay, ReplayArgs};
