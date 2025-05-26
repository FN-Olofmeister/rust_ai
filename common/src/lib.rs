//common/src/lib.rs

pub mod classifier;
pub mod email;

#[cfg(feature = "native")]
pub mod discord;
#[cfg(feature = "native")]
pub mod gmail;
