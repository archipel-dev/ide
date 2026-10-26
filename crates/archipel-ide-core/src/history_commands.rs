//! Local History commands (filled in with the history module).

use crate::IdeState;
use archipel_ide_command::CommandRegistry;

/// State of Local History in the app.
#[derive(Debug, Default)]
pub struct HistoryState {}

pub(crate) fn register(_registry: &mut CommandRegistry<IdeState>) {}
