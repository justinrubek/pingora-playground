use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    str::FromStr,
};

use crate::{
    commands::{Commands, ProxyCommands},
    error::Result,
};
use async_trait::async_trait;
use clap::Parser;
use hickory_resolver::{
    config::{NameServerConfig, ResolverConfig, ResolverOpts},
    name_server::TokioConnectionProvider,
    proto::{rr::rdata::SRV, xfer::Protocol},
    Resolver, TokioResolver,
};
use pingora::{http::StatusCode, prelude::*};

mod commands;
mod error;

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let args = commands::Args::parse();
    match args.command {
        Commands::Proxy(hello) => {
            let cmd = hello.command;
            match cmd {
                ProxyCommands::World => {
                    let mut server = Server::new(None).unwrap();
                    server.bootstrap();

                    let router = Router::new()?;

                    let mut proxy_service = http_proxy_service(&server.configuration, router);
                    proxy_service.add_tcp("0.0.0.0:8080");

                    server.add_service(proxy_service);

                    println!("Path-based router starting on port 8080");
                    println!("Route /a -> 10.1.1.151");
                    println!("Route /* -> 10.1.1.150");
                    server.run_forever();
                }
            }
        }
    }
}

pub struct Router {
    resolver: TokioResolver,
}

impl Router {
    pub fn new() -> Result<Self> {
        let mut config = ResolverConfig::new();
        let socket_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8053);
        let ns_config = NameServerConfig {
            socket_addr,
            protocol: Protocol::Udp,
            tls_dns_name: None,
            trust_negative_responses: true,
            bind_addr: None,
            http_endpoint: None,
        };
        config.add_name_server(ns_config);

        let resolver =
            Resolver::builder_with_config(config, TokioConnectionProvider::default()).build();

        Ok(Router { resolver })
    }

    fn get_service_name(&self, session: &Session) -> Option<String> {
        let path = session.req_header().uri.path();
        let host = match session.req_header().headers.get("host") {
            Some(h) => h.to_str().unwrap_or(""),
            None => "",
        };

        // TODO: robust implementation
        if path.starts_with("/service-a") {
            println!("service-a");
            return Some("service-a".to_string());
        }
        if path.starts_with("/service-b") {
            println!("service-b");
            return Some("service-b".to_string());
        }

        None
    }

    async fn srv_lookup(&self, service_name: &str) -> Result<(String, u16)> {
        let srv_name = format!("_{service_name}._tcp.example.com");
        let srv_result = self.resolver.srv_lookup(srv_name).await;
        println!("got srv result");

        match srv_result {
            Ok(srv_records) => {
                println!("got srv records");
                if let Some(record) = srv_records.iter().next() {
                    let target = record
                        .target()
                        .to_string()
                        .trim_end_matches('.')
                        .to_string();
                    let port = record.port();

                    return Ok((target, port));
                } else {
                    println!("did not find record :(");
                    Err(crate::error::Error::QuerySrvRecord)
                }
            }
            Err(_) => {
                println!("other case :(((");
                Err(crate::error::Error::QuerySrvRecord)
            }
        }
    }

    async fn a_lookup(&self, hostname: &str) -> Result<String> {
        let lookup_result = self.resolver.lookup_ip(hostname).await;

        match lookup_result {
            Ok(lookup) => {
                if let Some(ipv4) = lookup.iter().find(|ip| ip.is_ipv4()) {
                    Ok(ipv4.to_string())
                } else {
                    Err(crate::error::Error::QuerySrvRecord)
                }
            }
            Err(_) => Err(crate::error::Error::QuerySrvRecord),
        }
    }
}

#[async_trait]
impl ProxyHttp for Router {
    type CTX = ();
    fn new_ctx(&self) -> Self::CTX {}

    async fn upstream_peer(
        &self,
        session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> pingora::Result<Box<HttpPeer>> {
        let service_name = self.get_service_name(session).expect("invalid service");

        let (srv_target, srv_port) = self
            .srv_lookup(&service_name)
            .await
            .map_err(|e| Error::explain(ErrorType::ConnectNoRoute, "unable to find service"))?;

        let a_result = self
            .a_lookup(&srv_target)
            .await
            .map_err(|e| Error::explain(ErrorType::ConnectNoRoute, "unable to find service"))?;

        let ip = Ipv4Addr::from_str(&a_result).expect("bad ip");
        println!("ip: {ip}");
        let socket_addr = SocketAddr::new(IpAddr::V4(ip), srv_port);

        let peer = Box::new(HttpPeer::new(socket_addr, false, String::new()));
        return Ok(peer);
    }
}
