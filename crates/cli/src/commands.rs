#[derive(clap::Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub(crate) struct Args {
    #[clap(subcommand)]
    pub command: Commands,
}

#[derive(clap::Subcommand, Debug)]
pub(crate) enum Commands {
    Proxy(Proxy),
}

#[derive(clap::Args, Debug)]
pub(crate) struct Proxy {
    #[clap(subcommand)]
    pub command: ProxyCommands,
}

#[derive(clap::Subcommand, Debug)]
pub(crate) enum ProxyCommands {
    World,
}
