//! Identity crate – WireGuard pubkey as identity + OAuth token cache via
//! org.freedesktop.secrets. Zero passwords; the WireGuard handshake is the login.

pub mod gcloud_auth;
pub mod session;
pub mod wireguard;
pub mod token; // Keeping for now if needed internally

pub use session::{SessionManager, Session};
pub use gcloud_auth::GCloudAuth;
pub use wireguard::{WireGuardIdentity, PeerInfo};
pub use token::{TokenManager, CachedToken};