use clap_complete::Shell;

use crate::environment::Environment;

pub fn show_env_config(env: &'_ Environment, bare_path: bool, shell: Option<Shell>) {
    if bare_path {
        println!("{}", env.base_dir)
    } else if let Some(shell) = shell {
        match shell {
            Shell::Bash | Shell::Zsh => {
                println!("export PATH=\"{}:$PATH\"", env.base_dir)
            }
            Shell::Elvish => todo!(),
            Shell::Fish => println!("set -p PATH \"{}\"", env.base_dir),
            Shell::PowerShell => {
                println!("$env:Path += ';{}' ", env.base_dir)
            }
            _ => (),
        }
    }
}
