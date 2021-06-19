use super::errors::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Arch {
    X86_64,
    X86,
    Aarch64,
    Armv7,
    PowerPc,
    Mips,
    Mips64,
}

impl Arch {
    pub fn get() -> Result<Arch> {
        Ok(Arch::X86_64)
    }

    pub fn parse(s: &'_ str) -> Result<Arch> {
        match s.to_lowercase().as_str() {
            "x86_64" | "amd64" => Ok(Self::X86_64),
            "x86" | "i686" => Ok(Self::X86),
            _ => Err(MachineError::UnknownArch(s.to_owned())),
        }
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn test_arch_parse() -> Result<(), Box<dyn std::error::Error>> {
//         let arch = Arch::parse("amd64")?;
//         assert_eq!(arch, Arch::X86_64);

//         Ok(())
//     }
// }
