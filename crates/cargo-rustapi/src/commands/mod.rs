//! CLI commands

mod new;
mod run;
mod generate;
mod docs;

pub use new::{new_project, NewArgs};
pub use run::{run_dev, RunArgs};
pub use generate::{generate, GenerateArgs};
pub use docs::open_docs;
