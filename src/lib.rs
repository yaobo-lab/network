#![allow(non_upper_case_globals)]
#![allow(unused_variables)]

pub mod dto;
pub use dto::*;
pub mod utils;

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(feature = "wpa")]
#[cfg(target_os = "linux")]
pub(crate) mod wpa;

#[cfg(feature = "wifi")]
pub mod wifi;
