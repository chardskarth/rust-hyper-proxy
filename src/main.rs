// use httpmock::prelude::*;
// use serde_json::{json, Value};

use std::{convert::Infallible, net::SocketAddr};

use http_body_util::Full;
use hyper::{body::{Bytes, Incoming}, server, service::{service_fn, Service}, Method, Request, Response, StatusCode};
use hyper_util::{client::legacy::{connect::HttpConnector, Client}, rt::{TokioExecutor, TokioIo}};
use tokio::net::{TcpListener, TcpStream};

use hyper_proxy::{Proxy, ProxyConnector, Intercept};


type ClientBuilder = hyper::client::conn::http1::Builder;
type ServerBuilder = hyper::server::conn::http1::Builder;


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("Hello, world!");

    let addr = SocketAddr::from(([127, 0, 0, 1], 3200));

    let listener = TcpListener::bind(addr).await?;

    println!("Listening on http://{}", addr);

    let proxy_stream = TcpStream::connect("127.0.0.1:8000").await?;
    let proxy_io = TokioIo::new(proxy_stream);


    let anyyow = ClientBuilder::new().handshake(proxy_io).await;

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

    //
    // let server = MockServer::start();
    //
    // let m = server.mock(|when, then| {
    //     when.method(POST)
    //         .path("/users")
    //         .header("content-type", "application/json")
    //         .json_body(json!({ "name": "Fred" }));
    //     then.status(201)
    //         .header("content-type", "application/json")
    //         .json_body(json!({ "name": "Hans" }));
    // });
}

async fn handle_request(req: Request<Incoming>) -> Result<Response<Full<Bytes>>, Infallible> {
    println!("Received a request: {:#?}", req);


    // let proxy = {
    //     let proxy_uri = "http://my-proxy:8080".parse().unwrap();
    //     let proxy = Proxy::new(Intercept::All, proxy_uri);
    //     //proxy.set_authorization(Authorization::basic("John Doe", "Agent1234"));
    //     let connector = HttpConnector::new();
    //     let proxy_connector = ProxyConnector::from_proxy(connector, proxy).unwrap();
    //     proxy_connector
    // };
    
    // let client: Client<HttpConnector, TokioExecutor> = Client::builder(TokioExecutor::new())
    //     .http1_title_case_headers(true)
    //     .http1_preserve_header_case(true)
    //     .build_http();



    match (req.method(), req.uri().path()) {
        (&Method::GET, "/") => {
            Ok(Response::new(Full::new(Bytes::from("Hello, world!"))))
        },
        _ => {

            proxy(client.clone(), req).await

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

async fn proxy(_client: Client<HttpConnector, _>, req: Request<Incoming>) -> Result<Response<Full<Bytes>>, Infallible> {
    let headers = req.headers().clone();
    println!("headers: {:?}", headers);

    let path = req.uri().path().to_string();
    if path.starts_with("/hello") {
        let target_url = "http://127.0.0.1:8000".to_owned();
        let resp = get_response(_client, req, &target_url, &path).await?;
        return Ok(resp);
    }

    let resp = Response::new(Full::new(Bytes::from("sorry! no route found")));
    Ok(resp)
}

async fn get_response(client: Client<HttpConnector, _>, req: Request<Incoming>, target_url: &str, path: &str) -> Result<Response<Full<Bytes>>, Infallible> {
    let target_url = format!("{}{}", target_url, path);
    let headers = req.headers().clone();
    let mut request_builder = Request::builder()
        .method(req.method())
        .uri(target_url)
        .body(req.into_body())
        .unwrap();

    *request_builder.headers_mut() = headers;
    let response = client.request(request_builder).await?;
    //let body = hyper::body::to_bytes(response.into_body()).await?;
    //let body = );
    //let body = String::from_utf8(body.to_vec()).unwrap();

    let mut resp = Response::new(Full::new(Bytes::from(response.into_body())));
    *resp.status_mut() = StatusCode::OK;
    Ok(resp)
}
