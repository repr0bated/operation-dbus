//! State plugins - each manages a domain via native protocols
pub mod net;
pub mod systemd;

pub use net::NetStatePlugin;
pub use systemd::SystemdStatePlugin;
