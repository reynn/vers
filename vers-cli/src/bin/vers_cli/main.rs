//! Main entry point for VersCli

#![deny(warnings, missing_docs, trivial_casts, unused_qualifications)]
#![forbid(unsafe_code)]

use vers::application::APPLICATION;

/// Boot VersCli
fn main() {
    abscissa_core::boot(&APPLICATION);
}
