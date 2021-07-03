use super::errors::*;
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Os {
    Windows,
    Linux,
    Osx,
}

impl Os {
    /// Get the OS from the current system
    pub fn get() -> Result<Os> {
        if let Ok(system_os) = Self::get_from_system() {
            Ok(system_os)
        } else {
            let os = if cfg!(windows) {
                Os::Windows
            } else if cfg!(linux) {
                Os::Linux
            } else {
                Os::Osx
            };
            Ok(os)
        }
    }

    /// Parse a provide string and return the determined OS
    pub fn parse(s: &'_ str) -> Result<Self> {
        match s.trim().to_lowercase().as_str() {
            "windows" => Ok(Self::Windows),
            "linux" => Ok(Self::Linux),
            "mac" | "darwin" | "osx" | "os x" => Ok(Self::Osx),
            _ => Err(MachineError::UnknownOs(s.to_owned())),
        }
    }

    fn get_from_system() -> Result<Os> {
        if let Ok(uname_results) = Command::new("uname").output() {
            let name = String::from_utf8(uname_results.stdout).unwrap();
            Ok(Self::parse(&name)?)
        } else {
            Err(MachineError::UnknownOs("".into()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use test_case::test_case;

    #[test_case("Linux\n" ; "Normal Linux")]
    #[test_case("\tlinux\n" ; "Tabbed Linux")]
    fn os_parse_linux_test(name: &str) -> Result<()> {
        let os = Os::parse(name)?;
        assert_eq!(os, Os::Linux);
        Ok(())
    }

    #[test_case("Darwin\n" ; "Normal Darwin")]
    #[test_case("OS X\n" ; "Normal OS X")]
    fn os_parse_mac_test(name: &str) -> Result<()> {
        let os = Os::parse(name)?;
        assert_eq!(os, Os::Osx);
        Ok(())
    }
}
