use actix_web::{
    dev::PeerAddr, error, middleware, web, App, Error, HttpRequest, HttpResponse,
    HttpServer, post, get
};
use awc::Client;
use url::Url;
use log;

use std::env;

#[get("/list/{user}")]
async fn list_files(
    req: HttpRequest,
    client: web::Data<Client>,
    url: web::Data<Url>
) -> Result<HttpResponse, Error> {
    let mut new_url = (**url).clone();
    new_url.set_path(req.uri().path());
    new_url.set_query(req.uri().query());

    let mut g_req = client
        .request_from(new_url.as_str(), req.head())
        .no_decompress()
        .send()
        .await
        .unwrap();

    let body = g_req.body().await?;

    Ok(HttpResponse::Ok().body(body))
}

#[get("/download/{user}/{filename}")]
async fn download_file(
    req: HttpRequest,
    client: web::Data<Client>,
    url: web::Data<Url>,
    path: web::Path<(String, String)>,
) -> Result<HttpResponse, Error> {
    // let file_path = format!("{}/{}", &path.0.as_str(), &path.1.as_str());
    // let url = format!("http://localhost:8080/download/{}", file_path);

    let mut new_url = (**url).clone();
    new_url.set_path(req.uri().path());
    new_url.set_query(req.uri().query());

    log::info!(
        "{:?}", new_url
    );

    let mut g_req = client
        .request_from(new_url.as_str(), req.head())
        .no_decompress()
        .send()
        .await
        .unwrap();

    let body = g_req.body().await?;


    Ok(HttpResponse::Ok()
        .insert_header(("Content-Disposition", "attachment"))
        .body(body))

}

/// Forwards the incoming HTTP request using `awc`.
#[post("/upload")]
async fn forward_upload(
    req: HttpRequest,
    payload: web::Payload,
    peer_addr: Option<PeerAddr>,
    url: web::Data<Url>,
    client: web::Data<Client>,
) -> Result<HttpResponse, Error> {
    let mut new_url = (**url).clone();
    new_url.set_path(req.uri().path());
    new_url.set_query(req.uri().query());

    let forwarded_req = client
        .request_from(new_url.as_str(), req.head())
        .no_decompress();

    let forwarded_req = match peer_addr {
        Some(PeerAddr(addr)) => {
            forwarded_req.insert_header(("x-forwarded-for", addr.ip().to_string()))
        }
        None => forwarded_req,
    };

    let res = forwarded_req
        .send_stream(payload)
        .await
        .map_err(error::ErrorInternalServerError)?;

    let mut client_resp = HttpResponse::build(res.status());
    for (header_name, header_value) in res.headers().iter().filter(|(h, _)| *h != "connection") {
        client_resp.insert_header((header_name.clone(), header_value.clone()));
    }

    Ok(client_resp.streaming(res))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    // let flat_back_ip = match env::var("") {
    //     Ok(ip) => format!("http://flatbackservice", ip),
    //     Err(_) => panic!("No forward ip")
    // };

    let mut forward_url = Url::parse("http://flatbackservice").unwrap();
    // forward_url.set_host(Some("localhost")).unwrap();
    forward_url.set_port(Some(8080)).unwrap();

    log::info!("{}", forward_url);

    let reqwest_client = reqwest::Client::default();

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(Client::default()))
            .app_data(web::Data::new(reqwest_client.clone()))
            .app_data(web::Data::new(forward_url.clone()))
            .wrap(middleware::Logger::default())
            .service(forward_upload)
            .service(download_file)
            .service(list_files)
    })
    .bind("0.0.0.0:8080")?
    .workers(2)
    .run()
    .await
}
