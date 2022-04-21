use {crate::environment::Environment, std::fmt::Debug};

pub trait Shell: Debug {
    fn generate_exports(&self, env: &'_ Environment) -> crate::Result<String>;
    fn setup(&self, env: &'_ Environment) -> &'_ str;
}

pub fn get_shell() -> Box<dyn Shell> {
    if let Some(shell_env) = std::env::var_os("SHELL") {
        match shell_env
            .to_str()
            .unwrap_or_default()
            .split('/')
            .last()
            .unwrap_or_default()
        {
            "fish" => Box::new(Fish),
            "bash" => Box::new(Bash),
            "zsh" => Box::new(Zsh),
            _ => panic!("current shell is not currently supported"),
        }
    } else {
        panic!("Unable to determine which shell is being used")
    }
}

#[derive(Debug)]
pub struct Fish;

impl Shell for Fish {
    fn generate_exports(&self, env: &'_ Environment) -> crate::Result<String> {
        todo!()
    }

    fn setup(&self, env: &'_ Environment) -> &'_ str {
        todo!()
    }
}

#[derive(Debug)]
pub struct Bash;

impl Shell for Bash {
    fn generate_exports(&self, env: &'_ Environment) -> crate::Result<String> {
        todo!()
    }

    fn setup(&self, env: &'_ Environment) -> &'_ str {
        todo!()
    }
}

#[derive(Debug)]
pub struct Zsh;

impl Shell for Zsh {
    fn generate_exports(&self, env: &'_ Environment) -> crate::Result<String> {
        todo!()
    }

    fn setup(&self, env: &'_ Environment) -> &'_ str {
        todo!()
    }
}
