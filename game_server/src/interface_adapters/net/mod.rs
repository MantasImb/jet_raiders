// Network adapter modules split by external client sockets vs internal HTTP routes.

pub mod client;
pub mod internal;

pub use client::{spawn_lobby_serializers, ws_handler};
pub use internal::create_lobby_handler;
