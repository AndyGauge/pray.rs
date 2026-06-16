mod oauth;

pub use oauth::{CallbackPage, LoginPage};

#[cfg(feature = "ssr")]
pub use oauth::{google_client, facebook_client, OAuthConfig};
