use std::{
    convert::Infallible,
    net::SocketAddr,
    pin::Pin,
    task::{Context, Poll},
};

use http_body_util::{BodyExt, Full, combinators::BoxBody};
use hyper::{
    Method, Request, Response, StatusCode,
    body::{Body, Bytes, Frame, Incoming},
    client::conn::http1::SendRequest,
    server,
    service::{Service, service_fn},
};
use hyper_util::{
    client::legacy::{Client, connect::HttpConnector},
    rt::{TokioExecutor, TokioIo},
};
use tokio::net::{TcpListener, TcpStream};

use hyper_proxy::{Intercept, Proxy, ProxyConnector};

type ClientBuilder = hyper::client::conn::http1::Builder;
type ServerBuilder = hyper::server::conn::http1::Builder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("Hello, world!");

    let addr = SocketAddr::from(([127, 0, 0, 1], 3200));

    let listener = TcpListener::bind(addr).await?;

    println!("Listening on http://{}", addr);

    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);
        //let io = Read::poll_read(stream);
        //
        //

        tokio::task::spawn(async move {
            if let Err(err) = ServerBuilder::new()
                .preserve_header_case(true)
                .title_case_headers(true)
                .serve_connection(io, service_fn(handle_request))
                .with_upgrades()
                .await
            {
                println!("Failed to serve connection: {:?}", err);
            }
        });
    }
}

async fn handle_request(req: Request<Incoming>) -> Result<Response<Full<Bytes>>, hyper::Error> {
    println!("Received a request: {:#?}", req);

    match (req.method(), req.uri().path()) {
        // (&Method::GET, "/") => Ok(Response::new(Full::new(Bytes::from("Hello, world!")))),
        (&Method::GET, "/") => Ok(Response::new(Full::new(Bytes::from("Hello, world!")))),
        _ => {
            let proxy_stream = TcpStream::connect("127.0.0.1:8000").await;
            let proxy_io = TokioIo::new(proxy_stream);

            let (mut client, client_connection) = ClientBuilder::new().handshake(proxy_io).await?;

            tokio::task::spawn(async move {
                if let Err(err) = client_connection.await {
                    println!("Connection failed: {:?}", err);
                }
            });

            proxy(client, req)
                .await
                .and_then(async |a| a.collect().await?.aggregate())

            // })

            // Ok(
            //     Response::builder()
            //         .status(StatusCode::NOT_FOUND)
            //         .body(Full::new(Bytes::from("Not Found")))
            //         .unwrap()
            // )

            // let client = Client::builder(proxy);
            // let fut_http = client.request(req)
            //
            // Client::new()
            //     .get("https://jsonplaceholder.typicode.com/posts/1")
            //     .send()
            //     .await
        }
    }
}

async fn proxy(
    _client: SendRequest<Incoming>,
    req: Request<Incoming>,
) -> Result<Response<Incoming>, hyper::Error> {
    let headers = req.headers().clone();
    println!("headers: {:?}", headers);

    let path = req.uri().path().to_string();
    if path.starts_with("/hello") {
        let target_url = "http://127.0.0.1:8000".to_owned();
        let resp = get_response(_client, req, &target_url, &path).await?;
        return Ok(resp);
    } else {
        todo!();
    }

    // let resp = Response::new(GatewayBody::Incoming(Bytes::from("asdf")));
    // Ok(resp)
}

async fn get_response(
    mut client: SendRequest<Incoming>,
    req: Request<Incoming>,
    target_url: &str,
    path: &str,
) -> Result<Response<Incoming>, hyper::Error> {
    let target_url = format!("{}{}", target_url, path);
    let headers = req.headers().clone();
    let mut request_builder = Request::builder()
        .method(req.method())
        .uri(target_url)
        .body(req.into_body())
        .unwrap();

    *request_builder.headers_mut() = headers;
    let response = client.send_request(request_builder).await?;
    // let response = client.send_request(request_builder).await.and_then(|response| {
    //
    // })?;
    //let body = hyper::body::to_bytes(response.into_body()).await?;
    //let body = );
    //let body = String::from_utf8(body.to_vec()).unwrap();

    let mut resp = Response::new(response.into_body());
    *resp.status_mut() = StatusCode::OK;
    Ok(resp)
}

enum GatewayBody {
    Incoming(hyper::body::Incoming),
    Empty,
}

impl hyper::body::Body for GatewayBody {
    type Data = hyper::body::Bytes;
    type Error = hyper::Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        match &mut *self.get_mut() {
            Self::Incoming(incoming) => Pin::new(incoming).poll_frame(cx),
            Self::Empty => Poll::Ready(None),
        }
    }
}

struct MockBody {
    data: &'static [u8],
}

impl MockBody {
    fn new(data: &'static [u8]) -> Self {
        Self { data }
    }
}

impl Body for MockBody {
    type Data = Bytes;
    type Error = hyper::Error;

    fn poll_frame(
        mut self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Result<Frame<Bytes>, Self::Error>>> {
        if self.data.is_empty() {
            std::task::Poll::Ready(None)
        } else {
            let data = self.data;
            self.data = &[];
            std::task::Poll::Ready(Some(Ok(Frame::data(Bytes::from(data)))))
        }
    }
}
