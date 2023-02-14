extern crate env_logger;

use std::fs;
use std::io::Write;

use actix_files::NamedFile;
use actix_multipart::Multipart;
use actix_web::http::header::ContentDisposition;
use actix_web::{
    get, middleware::Logger, post, web, App, Error, HttpRequest, HttpResponse, HttpServer,
    Responder,
};
use futures_util::TryStreamExt as _;
use uuid::Uuid;

#[get("/list/{user}")]
async fn list_user_files(
    path: web::Path<String>
) -> HttpResponse {
    let paths = fs::read_dir(&path.as_str());
    let mut body = String::from("");

    match paths {
        Ok(paths) => {
            for file in paths {
                body = format!("{}{}\n", body, file.unwrap().file_name().to_str().unwrap());
            }
        },
        Err(_) => return HttpResponse::InternalServerError().body("Could not find user.")
    }

    HttpResponse::Ok().body(body)
}

#[get("/download/{user}/{filename}")]
async fn download_file(
    req: HttpRequest,
    path: web::Path<(String, String)>
) -> HttpResponse {
    let file_path = format!("{}/{}", &path.0.as_str(), &path.1.as_str());
    let file = NamedFile::open(&file_path.as_str());
    
    let content_disposition = ContentDisposition::attachment(path.1.as_str());

    match file {
        Ok(file) => file
            .set_content_disposition(content_disposition)
            .respond_to(&req),
        Err(_) => HttpResponse::NotFound().body(format!("File not found at {}", file_path.to_string())),
    }
}

#[post("/upload")]
async fn upload_file(req: HttpRequest, mut payload: Multipart) -> Result<HttpResponse, Error> {
    while let Some(mut field) = payload.try_next().await? {
        let content_disposition = field.content_disposition();
        
        let filename = content_disposition
            .get_filename()
            .map_or_else(|| Uuid::new_v4().to_string(), sanitize_filename::sanitize);
        let username = req.headers().get("Username").unwrap().to_str().ok().unwrap();
        
        fs::create_dir_all(format!("./{}", username)).unwrap();
        let filepath = format!("./{}/{}", username, filename);

        // File::create is blocking operation, use threadpool
        let mut f = web::block(|| std::fs::File::create(filepath)).await??;

        // Field in turn is stream of *Bytes* object
        while let Some(chunk) = field.try_next().await? {
            // filesystem operations are blocking, we have to use threadpool
            f = web::block(move || f.write_all(&chunk).map(|_| f)).await??;
        }
    }

    Ok(HttpResponse::Ok().body("File uploaded!"))
}

 
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "actix_web=info");
    env_logger::init();

    HttpServer::new(|| {
        App::new()
            .wrap(Logger::default())
            .service(download_file)
            .service(upload_file)
            .service(list_user_files)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
