//! Template engine commands and slot binding logic.

pub mod types;

#[cfg(test)]
mod tests;

use serde_json::Value;

use super::{CommandSpec, Enabled, Result};
use crate::Session;

pub use types::{SlotTag, SlotType, parse_slot_tag};

pub fn specs() -> Vec<CommandSpec> {
    vec![]
}
