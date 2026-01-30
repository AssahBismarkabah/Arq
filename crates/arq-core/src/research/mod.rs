mod document;
pub mod prompts;
mod runner;

pub use document::{Dependency, Finding, ResearchDoc, Source, SourceType};
pub use runner::{ResearchError, ResearchProgress, ResearchRunner};
