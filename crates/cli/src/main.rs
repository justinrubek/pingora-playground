use crate::{
    commands::{Commands, HelloCommands},
    error::Result,
};
use async_trait::async_trait;
use clap::Parser;
use pingora::prelude::*;

mod commands;
mod error;

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let args = commands::Args::parse();
    match args.command {
        Commands::Hello(hello) => {
            let cmd = hello.command;
            match cmd {
                HelloCommands::World => {
                    let mut server = Server::new(None).unwrap();
                    server.bootstrap();

                    let mut proxy_service = http_proxy_service(&server.configuration, Router);
                    proxy_service.add_tcp("0.0.0.0:8080");

                    server.add_service(proxy_service);

                    println!("Path-based router starting on port 8080");
                    println!("Route /a -> 10.1.1.151");
                    println!("Route /* -> 10.1.1.150");
                    server.run_forever();
                }
                HelloCommands::Name { name } => {
                    println!("Hello, {name}!");
                }
                HelloCommands::Error => {
                    Err(crate::error::Error::Other("error".into()))?;
                }
            }
        }
    }

    Ok(())
}

pub struct Router;

#[async_trait]
impl ProxyHttp for Router {
    type CTX = ();
    fn new_ctx(&self) -> Self::CTX {}

    async fn upstream_peer(
        &self,
        session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> pingora::Result<Box<HttpPeer>> {
        let path = session.req_header().uri.path();

        let upstream = match path {
            "/a" => "10.1.1.151:80",
            _ => "10.1.1.150:80",
        };

        println!("Path: {}, routing to: {}", path, upstream);

        let peer = Box::new(HttpPeer::new(upstream, false, String::new()));
        Ok(peer)
    }
}
