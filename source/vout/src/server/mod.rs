use std::ops::Add;
use std::sync::Arc;

use hyper::service::service_fn;
use hyper::{Request, server::conn::http1};
use hyper_util::rt::TokioIo;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::crypto::tls;
use crate::server::res::{GwPassed, GwResponseInner};
use crate::{config, errorln, outputln};

pub mod err;
pub mod res;
pub mod service;

pub const AWS_HEADER_REQUEST_ID: &str = "x-amzn-RequestId";
pub const AMZ_TARGET_HEADER: &str = "X-Amz-Target";

struct AmznTargetFriendly(Option<String>);

impl std::fmt::Debug for AmznTargetFriendly {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            Some(value) => write!(f, "{}", value),
            None => write!(f, "N/A"),
        }
    }
}

async fn dispatch_handler<'a>(
    req: Request<hyper::body::Incoming>,
) -> (anyhow::Result<GwPassed<'a>>, u128, AmznTargetFriendly) {
    let path = req.uri().path().to_owned();
    let method = req.method().clone();
    let target = req
        .headers()
        .get(AMZ_TARGET_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);

    let start_time = std::time::Instant::now();

    let result = match (target.as_deref(), path.as_str(), &method) {
        (None, "/health", &hyper::Method::GET) => service::health::handler().await,
        (None, "/ready", &hyper::Method::GET) => service::ready::handler().await,
        _ => Ok(err::builder::unknown_route()),
    };

    let elapsed = start_time.elapsed().as_millis();

    (result, elapsed, AmznTargetFriendly(target))
}

async fn gateway(
    address: String,
    req: Request<hyper::body::Incoming>,
) -> anyhow::Result<GwResponseInner> {
    let path = req.uri().path().to_owned();
    let method = req.method().clone();
    let request_id = Uuid::new_v4();
    let (result, elapsed, amzn_target) = dispatch_handler(req).await;

    match result {
        Ok(v) => {
            let response = match &v {
                GwPassed::Success(_) => "OK".to_owned(),
                GwPassed::Failure((code, message)) => format!("{code}/{message}"),
            };

            outputln!(
                "request received", id: request_id, request: address, method: method, path: path, target: amzn_target, duration: elapsed, response: response
            );

            v.into_response(request_id)
        }
        Err(e) => {
            errorln!(
                "request failed", id: request_id, request: address, method: method, path: path, target: amzn_target, duration: elapsed, response: format!("{} ({:?})", err::INTERNAL_SERVICE_ERROR, e)
            );

            err::builder::internal_service_error().into_response(request_id)
        }
    }
}

pub async fn start() -> anyhow::Result<()> {
    let config = config::get();
    let server_config = &config.server;
    let listen_address = &server_config.listen_address;

    outputln!("starting gateway service", address: listen_address);

    let tls_acceptor = Arc::new(RwLock::new(None));

    if server_config.tls.is_some() {
        let tls_acceptor = tls_acceptor.clone();
        let result = tls::renew_spin().await;

        *tls_acceptor.write().await =
            Some(tokio_rustls::TlsAcceptor::from(Arc::new(result.config)));

        let mut not_after = result.not_after;

        tokio::spawn(async move {
            loop {
                let sleep_len = not_after - chrono::Utc::now().timestamp();
                tokio::time::sleep(tokio::time::Duration::from_secs(sleep_len as u64)).await;

                outputln!("TLS certificate renewer thread woke up");

                let result = tls::renew_spin().await;

                *tls_acceptor.write().await =
                    Some(tokio_rustls::TlsAcceptor::from(Arc::new(result.config)));

                not_after = result.not_after;

                outputln!("TLS certificate renewer thread successfully renewed certificate", not_after: chrono::Utc::now()
                    .add(chrono::Duration::seconds(not_after as i64))
                    .to_rfc2822());
            }
        });
    } else {
        *tls_acceptor.write().await = None;
    }

    let listener = tokio::net::TcpListener::bind(listen_address).await?;

    loop {
        let tls_acceptor = tls_acceptor.clone();
        let (stream, addr) = listener.accept().await?;

        tokio::spawn(async move {
            if let Some(tls_acceptor) = tls_acceptor.read().await.as_ref() {
                let stream = match tls_acceptor.accept(stream).await {
                    Ok(stream) => stream,
                    Err(err) => {
                        errorln!("error accepting TLS connection", error: err);
                        return;
                    }
                };

                let io = TokioIo::new(stream);

                if let Err(err) = http1::Builder::new()
                    .serve_connection(io, service_fn(|req| gateway(addr.to_string(), req)))
                    .await
                {
                    errorln!("error serving connection", error: err);
                }
            } else {
                let io = TokioIo::new(stream);

                if let Err(err) = http1::Builder::new()
                    .serve_connection(io, service_fn(|req| gateway(addr.to_string(), req)))
                    .await
                {
                    errorln!("error serving connection", error: err);
                }
            }
        });
    }
}
