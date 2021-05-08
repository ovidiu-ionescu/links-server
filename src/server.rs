use std::convert::Infallible;
use bytes::Buf;
use std::{ fs::File, io::{BufWriter, Write}, };

use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};

use config::{Config, File as ConfigFile, FileFormat};
use std::net::SocketAddr;

use serde::{Deserialize, Serialize};

use lazy_static::lazy_static;
type GenericError = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = std::result::Result<T, GenericError>;

lazy_static! {
    static ref CONFIG: ApConfig = ApConfig::read_config();
}

#[derive(Serialize, Deserialize, Debug)]
struct Payload {
    uuid: String,
    content: String,
}

struct ApConfig {
    address: String,
    storage_dir: String,
}

impl ApConfig {
    fn read_config() -> ApConfig {
        let mut c = Config::new();
        c.merge(ConfigFile::new("settings", FileFormat::Toml).required(true)).unwrap();
        let address = c.get_str("address").unwrap();
        let storage_dir = c.get_str("storage_dir").unwrap();

        ApConfig { address, storage_dir, }
    }   
}

async fn do_work(p: Payload) -> Result<String> {
    println!("Json received: {:#?}", p);
    let file_name = format!("{}/{}.md", CONFIG.storage_dir, p.uuid);
    let file = File::create(file_name)?;
    let mut out = BufWriter::new(&file);
    write!(out, "{}", p.content)?;

    Ok(String::from("Standard response"))
}

async fn request_handler(req: Request<Body>) -> Result<Response<Body>> {
    match (req.method(), req.uri().path()) {
        (&Method::POST, "/save_links") => {
            //let b = hyper::body::to_bytes(req).await?;
            //let p = serde_json::from_reader(b)?;

            let whole_body = hyper::body::aggregate(req).await?;
            let p: Payload = serde_json::from_reader(whole_body.reader())?;

            let content = do_work(p).await?;

            let response = Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "text/html")
                .header("server", "hyper")
                .body(Body::from(content))
                .unwrap();
            Ok(response)
        }
        _ => {
            let mut not_found = Response::default();
            *not_found.status_mut() = StatusCode::NOT_FOUND;
            Ok(not_found)
        }
    }   
}

pub async fn start_server() -> Result<()> {
    pretty_env_logger::init();

    // For every connection, we must make a `Service` to handle all
    // incoming HTTP requests on said connection.
    let make_svc = make_service_fn(|_conn| {
        // This is the `Service` that will handle the connection.
        // `service_fn` is a helper to convert a function that
        // returns a Response into a `Service`.
        async { Ok::<_, Infallible>(service_fn(request_handler)) }
    }); 

    let socket_addr: SocketAddr = CONFIG.address.parse().expect("Unble to parse socket address");
    let server = Server::bind(&socket_addr).serve(make_svc);

    println!("Listening on http://{}", &CONFIG.address);

    server.await?;

    Ok(())
}
