use actix_web::{get, web, App, Result, HttpServer, HttpResponse, Responder};
use color_eyre::eyre;
use futures::Future;
use std::{net::ToSocketAddrs, pin::Pin};

#[get("/verify/{payload}")]
async fn check(payload: web::Path<String>, data: web::Data<AppStateCbWithAsyncFn>) -> impl Responder 
{
    let inner = payload.into_inner();
    
    let bytes = inner.split(",")
    .map(|v| v.parse::<u8>())
    .filter_map(Result::ok)
    .collect::<Vec<u8>>();

    let verify = &data.as_ref().cb;
    let _ =  verify(bytes).await;
    
    HttpResponse::Ok()
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