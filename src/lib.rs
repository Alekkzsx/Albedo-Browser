#![warn(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::dbg_macro,
    clippy::panic,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::module_name_repetitions,
    clippy::must_use_candidate,
    clippy::struct_excessive_bools,
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    clippy::manual_string_new
)]
#![deny(
    clippy::allow_attributes_without_reason,
    clippy::incorrect_self_convention,
    clippy::rc_buffer,
    clippy::multiple_crate_versions
)]

pub mod ace;
pub mod browser;
pub mod network;
pub mod renderer;
pub mod ui;
pub mod utils;

