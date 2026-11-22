//! Domain services: business rules on top of the database.
//! Each function takes a connection so it can be unit-tested with an in-memory DB.

pub mod backup;
pub mod notes;
pub mod reminders;
pub mod settings;
pub mod tasks;
pub mod timer;
pub mod workspaces;
