use std::{collections::HashMap, sync::Arc};

use serde_json::{Map, Value};
use thiserror::Error;

use crate::{state::AppState, ViewId, WindowId};

pub type JsonMap = Map<String, Value>;
pub type CommandHandler = Arc<dyn Fn(&mut AppState, CommandInvocation) -> CommandResult + Send + Sync>;
pub type CommandResult = Result<CommandOutput, CommandError>;

#[derive(Debug, Clone)]
pub enum CommandTarget {
    Application,
    Window(WindowId),
    View(ViewId),
}

#[derive(Debug, Clone)]
pub struct CommandInvocation {
    pub target: CommandTarget,
    pub args: JsonMap,
}

impl CommandInvocation {
    pub fn new(target: CommandTarget) -> Self {
        Self {
            target,
            args: JsonMap::new(),
        }
    }

    pub fn with_args(mut self, args: JsonMap) -> Self {
        self.args = args;
        self
    }
}

#[derive(Debug, Clone)]
pub struct CommandOutput {
    pub message: Option<String>,
}

impl CommandOutput {
    pub fn ok(message: impl Into<String>) -> Self {
        Self {
            message: Some(message.into()),
        }
    }

    pub fn empty() -> Self {
        Self { message: None }
    }
}

#[derive(Debug, Clone)]
pub struct CommandSpec {
    pub name: String,
    pub description: String,
}

impl CommandSpec {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
        }
    }
}

#[derive(Clone)]
struct RegisteredCommand {
    spec: CommandSpec,
    handler: CommandHandler,
}

#[derive(Default)]
pub struct CommandBus {
    commands: HashMap<String, RegisteredCommand>,
}

impl CommandBus {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, spec: CommandSpec, handler: CommandHandler) {
        self.commands.insert(
            spec.name.clone(),
            RegisteredCommand { spec, handler },
        );
    }

    pub fn handler(&self, name: &str) -> Option<CommandHandler> {
        self.commands.get(name).map(|command| Arc::clone(&command.handler))
    }

    pub fn specs(&self) -> Vec<CommandSpec> {
        let mut specs: Vec<_> = self.commands.values().map(|command| command.spec.clone()).collect();
        specs.sort_by(|a, b| a.name.cmp(&b.name));
        specs
    }

    pub fn contains(&self, name: &str) -> bool {
        self.commands.contains_key(name)
    }
}

#[derive(Debug, Error)]
pub enum CommandError {
    #[error("command not found: {0}")]
    NotFound(String),
    #[error("missing argument: {0}")]
    MissingArgument(String),
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
    #[error("no active view")]
    NoActiveView,
    #[error("buffer operation failed: {0}")]
    Buffer(String),
}
