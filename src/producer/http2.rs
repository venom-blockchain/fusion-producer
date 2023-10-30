use std::{
    net::SocketAddr,
    pin::Pin,
    task::{Context, Poll},
};

use anyhow::Result;
use futures_util::Future;
use hyper::{service::Service, Body, Request, Response, Server, StatusCode};
use tokio::sync::broadcast::Receiver;
use tokio_stream::wrappers::BroadcastStream;

use super::TransportData;

pub fn start_producer_service(receiver: Receiver<TransportData>, listen_address: SocketAddr) {
    tokio::spawn(async move {
        tracing::info!("Starting http/2 transport server on: {}", &listen_address);

        let server = Server::bind(&listen_address)
            .http2_only(true)
            .serve(MakeProducerService { receiver });

        if let Err(error) = server.await {
            tracing::error!("Http2 producer: {}", error);
        }
    });
}

struct ProducerService {
    messages_receiver: Receiver<TransportData>,
}

impl Service<Request<Body>> for ProducerService {
    type Response = Response<Body>;
    type Error = hyper::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _: &mut Context) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        fn ok_response(s: String) -> Result<Response<Body>, hyper::Error> {
            Ok(Response::builder().body(Body::from(s)).unwrap())
        }
        fn response_error(status: StatusCode) -> Result<Response<Body>, hyper::Error> {
            Ok(Response::builder().status(status).body(Body::empty()).unwrap())
        }

        let res = match req.uri().path() {
            "/" => ok_response("Subscribe to one of the streams".to_string()),
            "/messages/data" => {
                // TODO: This might discard some messages (look up resubscribe)
                let mut receiver = self.messages_receiver.resubscribe();
                std::mem::swap(&mut self.messages_receiver, &mut receiver);
                let stream = BroadcastStream::new(receiver);
                let body: Body = Body::wrap_stream(stream);
                Ok(Response::new(body))
            },
            _ => response_error(StatusCode::NOT_FOUND),
        };

        Box::pin(async { res })
    }
}

struct MakeProducerService {
    receiver: Receiver<TransportData>,
}

impl<T> Service<T> for MakeProducerService {
    type Response = ProducerService;
    type Error = hyper::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _: &mut Context) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _: T) -> Self::Future {
        let mut receiver = self.receiver.resubscribe();
        std::mem::swap(&mut self.receiver, &mut receiver);
        let fut = async move {
            Ok(ProducerService {
                messages_receiver: receiver,
            }) 
        };
        Box::pin(fut)
    }
}

