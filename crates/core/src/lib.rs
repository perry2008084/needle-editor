pub mod app;
pub mod buffer;
pub mod command;
pub mod history;
pub mod ids;
pub mod selection;
pub mod settings;
pub mod state;
pub mod view;

pub use app::NeedleApp;
pub use buffer::{Buffer, BufferError};
pub use command::{
    CommandBus, CommandError, CommandHandler, CommandInvocation, CommandOutput, CommandSpec,
    CommandTarget,
};
pub use history::{EditRecord, EditTransaction};
pub use ids::{BufferId, ViewId, WindowId};
pub use selection::{Region, SelectionSet};
pub use settings::Settings;
pub use state::AppState;
pub use view::View;
