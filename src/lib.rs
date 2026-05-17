pub mod common;
pub mod engines;

pub mod genie {
    #![allow(
        non_upper_case_globals,
        non_camel_case_types,
        non_snake_case,
        dead_code,
        clippy::all
    )]
    include!(concat!(env!("OUT_DIR"), "/genie_bindings.rs"));
}
