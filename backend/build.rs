//! Build script for the backend crate.
//!
//! When the `embedded-frontend` feature is enabled, verify that
//! `frontend/dist/index.html` exists (it is produced by
//! `trunk build --release`) so the `include_dir!` macro has something to
//! embed at compile time. The build script does *not* invoke trunk itself —
//! CI and Dockerfiles run trunk as an explicit prior step.

fn main() {
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_EMBEDDED_FRONTEND");

    if std::env::var_os("CARGO_FEATURE_EMBEDDED_FRONTEND").is_none() {
        return;
    }

    let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let dist = std::path::PathBuf::from(&manifest)
        .join("..")
        .join("frontend")
        .join("dist");
    let index = dist.join("index.html");

    if !index.exists() {
        eprintln!(
            "\nerror: feature `embedded-frontend` is enabled but {} was not found.",
            index.display()
        );
        eprintln!(
            "       run `cd frontend && trunk build --release` before building \
             the backend with this feature.\n"
        );
        std::process::exit(1);
    }

    // cargo scans the path recursively for changes when it is a directory, so
    // a single rerun-if-changed line covers every emitted asset under dist/.
    println!("cargo:rerun-if-changed={}", dist.display());
}
