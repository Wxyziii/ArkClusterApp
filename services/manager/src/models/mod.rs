//! Data models for the manager. Phase 1 keeps control-plane models
//! (systemd, rcon, governor) as inert data/abstractions only.

pub mod audit;
pub mod domain;
pub mod governor;
pub mod rcon;
pub mod systemd;
