#![feature(async_closure)]

use tokio::net::TcpListener;
use tokio::net::TcpStream;
use std::net::SocketAddr;
use std::convert::Infallible;
use hyper::{Body, Request, Response, Server};
use hyper::service::{make_service_fn, service_fn};
use std::pin::Pin;
use std::future::Future;

pub struct HttpTransport {
    pub address: String,
    pub socket: Option<(TcpStream, SocketAddr)>
}

impl HttpTransport {
    pub async fn new(addr: &str) -> Self {
        Self {
            address: addr.to_string(),
            socket: None
        }
    }

    pub async fn accept(&mut self) -> () {
        if let Ok(l) = TcpListener::bind(&self.address).await {
            self.socket = l.accept().await.ok();
        }
    }

    pub async fn listen(&self, handler: &'static fn(Request<Body>)->Result<Response<Body>, Infallible>) -> () {
        let make_svc = make_service_fn(|_conn| async {
            // service_fn converts our function into a `Service`
            Ok::<_, Infallible>(service_fn(|req| async {
                handler(req)
            }))
        });
        return match &self.socket {
            Some(s) => {
                Server::bind(&s.1).serve(make_svc);
            },
            _ => {
                panic!("Failed on binding server")
            }
        }
    }
}
