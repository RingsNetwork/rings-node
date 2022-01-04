use std::net::SocketAddr;
use std::convert::Infallible;
use bns_node::http_transport::HttpTransport;
use bns_node::ice_transport::IceTransport;
use hyper::service::{make_service_fn, service_fn};
use hyper::Server;
use std::sync::Arc;


#[tokio::main]
async fn main() {
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    let make_svc = make_service_fn(|_conn| async {
        // service_fn converts our function into a `Service`
        Ok::<_, Infallible>(service_fn(|req| async {
            let ice_transport = IceTransport::new().await;
            let http_transport = HttpTransport::new("127.0.0.1:9999", ice_transport).await;
            http_transport.handler(req).await
        }))
    });

    let server = Server::bind(&addr).serve(make_svc);
    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}
