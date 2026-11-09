// Copyright (c) Microsoft Corporation. All rights reserved.

/// Only the opt-in `native-tls` forwarding backend links the target's OpenSSL;
/// Rustls forwarding and archive acquisition need no OpenSSL at all.
pub(super) fn require_target_provider() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("linux")
        || std::env::var_os("CARGO_FEATURE_NATIVE_TLS").is_none()
    {
        return;
    }
    let target = std::env::var("TARGET").expect("Cargo provides TARGET");
    // Runtime bindings attest their loader contract through Cargo metadata;
    // a linked provider needs explicit shared-linkage overrides. Check target
    // vendoring independently in either case.
    let runtime_bindings = runtime_bindings();
    require_dynamic_openssl("Linux SDK target", &target, runtime_bindings);
    let requirement = if runtime_bindings {
        "runtime-loaded OpenSSL API profile 1.1.1w"
    } else {
        "OS OpenSSL 3"
    };
    let version = std::env::var("DEP_OPENSSL_VERSION_NUMBER").unwrap_or_else(|error| {
        panic!("Linux SDK target requires {requirement} version metadata: {error}")
    });
    let version = i64::from_str_radix(&version, 16).unwrap_or_else(|error| {
        panic!("Linux SDK target requires {requirement} version metadata: {error}")
    });
    if runtime_bindings {
        assert_eq!(
            version, 0x1010_117f,
            "Linux SDK target requires {requirement}; found version {version:#x}"
        );
    } else {
        require_openssl_3("Linux SDK target", version);
    }
}

fn runtime_bindings() -> bool {
    std::env::var("DEP_OPENSSL_RUNTIME_BINDINGS").as_deref() == Ok("1")
}

pub(super) fn require_openssl_3(boundary: &str, version: i64) {
    assert!(
        (0x3000_0000..0x4000_0000).contains(&version),
        "{boundary} requires OS OpenSSL 3; found version {version:#x}"
    );
}

fn require_dynamic_openssl(boundary: &str, triple: &str, runtime_bindings: bool) {
    let prefix = triple.to_ascii_uppercase().replace('-', "_");
    let requirement = if runtime_bindings {
        "non-vendored runtime OpenSSL bindings"
    } else {
        "OS OpenSSL 3"
    };
    for (name, expected) in [("OPENSSL_STATIC", "0"), ("OPENSSL_NO_VENDOR", "1")] {
        let prefixed = format!("{prefix}_{name}");
        println!("cargo:rerun-if-env-changed={name}");
        println!("cargo:rerun-if-env-changed={prefixed}");
        let value = std::env::var_os(&prefixed).or_else(|| std::env::var_os(name));
        assert!(
            value.as_deref() == Some(std::ffi::OsStr::new(expected))
                || (value.is_none() && runtime_bindings),
            "{boundary} requires {requirement} without static linkage; set {name}={expected} (or {prefixed})"
        );
    }
    println!("cargo:rerun-if-env-changed=DEP_OPENSSL_VENDORED");
    assert!(
        std::env::var_os("DEP_OPENSSL_VENDORED").is_none(),
        "{boundary} requires {requirement}, not vendored OpenSSL"
    );
}
