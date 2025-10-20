//! State plugins - each manages a domain via native protocols
pub mod login1;
pub mod lxc;
pub mod net;
pub mod systemd;

pub use login1::Login1Plugin;
pub use lxc::LxcPlugin;
pub use net::NetStatePlugin;
pub use systemd::SystemdStatePlugin;
