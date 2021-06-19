use super::errors::*;
use log::*;
use std::{ascii::AsciiExt, process::Command};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Os {
    Windows,
    Linux,
    Osx,
}

impl Os {
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

    fn get_from_system() -> Result<Os> {
        if let Ok(uname_results) = Command::new("uname").output() {
            let name = String::from_utf8(uname_results.stdout).unwrap();
            Ok(Self::parse(&name)?)
        } else {
            Err(MachineError::UnknownOs("".into()))
        }
    }

    pub fn parse(s: &'_ str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "windows" => Ok(Self::Windows),
            "linux" => Ok(Self::Linux),
            "mac" | "osx" => Ok(Self::Osx),
            _ => Err(MachineError::UnknownOs(s.to_owned())),
        }
    }
}
