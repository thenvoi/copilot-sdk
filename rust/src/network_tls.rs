// Copyright (c) Microsoft Corporation. All rights reserved.

//! Client-local TLS with platform certificate policy.

use rustls_platform_verifier::BuilderVerifierExt;
use tokio_rustls::rustls;

pub(crate) fn client_config() -> Result<rustls::ClientConfig, rustls::Error> {
    #[cfg(feature = "rustls")]
    let provider = std::sync::Arc::new(rustls::crypto::aws_lc_rs::default_provider());
    #[cfg(not(feature = "rustls"))]
    let provider = rustls::crypto::CryptoProvider::get_default()
        .cloned()
        .ok_or_else(|| {
            rustls::Error::General(
                "Install a rustls crypto provider before forwarding requests".into(),
            )
        })?;
    rustls::ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()?
        .with_platform_verifier()
        .map(|builder| builder.with_no_client_auth())
}
