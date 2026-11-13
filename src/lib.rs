//! Hull is a hosted web app shell.
//! Cards and pipes are JSON. The airlock is default-deny. Nothing here opens an outbound socket.
//! `radio` is the phase-1 Google Chat contract. The deck renders it. The behavior is still red.

pub mod airlock;
mod embed;
pub mod gate;
pub mod http;
pub mod hull;
pub mod json;
pub mod load;
pub mod radio;
mod render;
pub mod schema;
pub mod session;
pub mod sync;

#[cfg(not(target_arch = "wasm32"))]
pub mod server;

#[cfg(target_arch = "wasm32")]
mod wasm;

pub use http::{Request, Response};
pub use hull::Hull;
