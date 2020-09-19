use actix_files::NamedFile;
use actix_web::{get, web, App, Result, HttpServer};
use color_eyre::eyre;
use futures::Future;
use std::{path::PathBuf, net::ToSocketAddrs, pin::Pin};

#[get("/verify/{payload}")]
async fn check(payload: web::Path<String>, data: web::Data<AppStateCbWithAsyncFn>) -> actix_web::Result<NamedFile> 
{
    let inner = payload.into_inner();
    
    let bytes = inner.split(",")
    .map(|v| v.parse::<u8>())
    .filter_map(Result::ok)
    .collect::<Vec<u8>>();

    let verify = &data.as_ref().cb;
    let _ =  verify(bytes).await;
    
    let path = PathBuf::from("mcmonarch_web/static/index.html");
    Ok(NamedFile::open(path)?)
}

struct AppStateCbWithAsyncFn {
    cb: Box<dyn Sync + Send + Fn(Vec<u8>) -> Pin<Box<dyn Future<Output = eyre::Result<()>> + Send>>>
}

pub async fn get_web(addr: impl ToSocketAddrs, verify_cb: Box<dyn Sync + Send + Fn(Vec<u8>) -> Pin<Box<dyn Future<Output = eyre::Result<()>> + Send >>> ) -> eyre::Result<()> 
{
    let async_handler = web::Data::new(AppStateCbWithAsyncFn {
        cb: verify_cb
    });
    
    HttpServer::new(move || {
            App::new()
                .app_data(async_handler.clone())
                .service(check)
    })
    .disable_signals()
    .bind(addr)
    .map_err(|e| eyre::eyre!(e))?
    .run()
    .await
    .map(|_| ())
    .map_err(|e| eyre::eyre!(e))
}