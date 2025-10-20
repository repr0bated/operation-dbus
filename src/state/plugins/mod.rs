//! State plugins - each manages a domain via native protocols
pub mod net;
pub mod systemd;
pub mod login1;
pub mod lxc;

pub use net::NetStatePlugin;
pub use systemd::SystemdStatePlugin;
pub use login1::Login1Plugin;
pub use lxc::LxcPlugin;
