//! Stable bounded-client facade for the external standalone PG renderer.
//!
//! The private client owns the shipped form protocol and response projection;
//! consumers retain these public renderer types at their original path.

#[path = "http_renderer/client.rs"]
mod client;
#[path = "http_renderer/grade.rs"]
mod grade;
#[path = "http_renderer/protocol.rs"]
mod protocol;
#[path = "http_renderer/response_shape.rs"]
mod response_shape;

pub use client::{HttpWebworkRenderer, HttpWebworkRendererConfig, RendererConfigError};
