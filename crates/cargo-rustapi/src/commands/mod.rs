//! CLI commands

mod docs;
mod generate;
mod new;
mod run;

pub use docs::open_docs;
pub use generate::{generate, GenerateArgs};
pub use new::{new_project, NewArgs};
pub use run::{run_dev, RunArgs};
