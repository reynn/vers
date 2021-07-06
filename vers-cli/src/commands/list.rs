use crate::prelude::*;
use structopt::{clap::arg_enum, StructOpt};

#[derive(Debug, Clone, StructOpt)]
pub struct ListCmd {
    #[structopt(short, long, aliases = &["env"])]
    pub environment: Option<String>,
    #[structopt(short, long)]
    pub output: Option<ListOutputType>,
}

arg_enum! {
    #[derive(Debug, Clone)]
    pub enum ListOutputType {
        Json,
        Yaml,
        Text,
    }
}

impl From<ListOutputType> for OutputType {
    fn from(val: ListOutputType) -> Self {
        match val {
            ListOutputType::Json => OutputType::Json,
            ListOutputType::Yaml => OutputType::Yaml,
            ListOutputType::Text => OutputType::Text,
        }
    }
}

impl Default for ListOutputType {
    fn default() -> Self {
        Self::Text
    }
}

pub(crate) fn execute_list_command(
    args: &'_ ListCmd,
    environment: &'_ Environment,
    _cfg: &'_ Config,
) -> Result<()> {
    let output_type: OutputType = args.clone().output.unwrap_or_default().into();
    log::info!("Listing in {:?} format", output_type);
    vers_core::list_tools(&ListToolsOpts {
        environment: &environment,
        output_type: &output_type,
    })?;
    Ok(())
}
