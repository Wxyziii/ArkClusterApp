//! Data models for the manager. Phase 1 keeps control-plane models
//! (systemd, rcon, governor) as inert data/abstractions only.

pub mod audit;
pub mod backup;
pub mod config_edit;
pub mod domain;
pub mod governor;
pub mod maintenance;
pub mod mods;
pub mod node_tasks;
pub mod nodes;
pub mod operations;
pub mod rcon;
pub mod resources;
pub mod runtime;
pub mod systemd;
pub mod travel;
pub mod travel_sessions;
