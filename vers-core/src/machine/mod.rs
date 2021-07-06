pub(crate) mod arch;
pub(crate) mod errors;
pub(crate) mod os;
pub(crate) mod os_version;

use self::{arch::Arch, os::Os};
pub use errors::*;
use thiserror::Error;

#[derive(Debug, Clone, Error)]
/// TODO: write docs
pub enum DetailsError {}

#[derive(Debug, Clone)]
/// TODO: write docs
pub struct Details {
    /// TODO: write docs
    pub os: Os,
    /// TODO: write docs
    pub arch: Arch,
}

impl Details {
    /// TODO: write docs
    pub fn get() -> Result<Details> {
        Ok(Details {
            os: Self::get_os_details()?,
            arch: Self::get_arch()?,
        })
    }

    fn get_os_details() -> Result<Os> {
        Os::get()
    }

    fn get_arch() -> Result<Arch> {
        Arch::get()
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn test_os_parse() {
//         let os = Os::parse("windows").expect("Windows was not a recognized OS");
//         assert_eq!(os, Os::Windows);
//         let os = Os::parse("linux").expect("Linux was not a recognized OS");
//         assert_eq!(os, Os::Linux);
//         let os = Os::parse("mac").expect("Mac was not a recognized OS");
//         assert_eq!(os, Os::Osx);
//         let os = Os::parse("osx").expect("Mac was not a recognized OS");
//         assert_eq!(os, Os::Osx);
//     }

//     #[test]
//     fn test_get_os_details() {
//         let os_details = Details::get().expect("Failed to get the details for current machine");
//         assert_eq!(&os_details.os, &Os::Linux);
//         assert_eq!(&os_details.arch, &Arch::X86_64);
//     }
// }
