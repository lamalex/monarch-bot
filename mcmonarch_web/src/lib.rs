use actix_web::{get, web, App, Result, HttpServer, HttpResponse, Responder};
use tokio::sync::mpsc;
use std::net::ToSocketAddrs;

#[get("/verify/{payload}")]
async fn check(payload: web::Path<String>, data: web::Data<mpsc::UnboundedSender<String>>) -> impl Responder {
    match data.send(payload.clone()) {
        Ok(_) => println!("payload {:?}", payload),
        Err(f) => println!("{:?}", f)
    };

    HttpResponse::Ok()
}

pub async fn get_web(addr: impl ToSocketAddrs, tx: mpsc::UnboundedSender<String>) -> Result<(), String> {
    let handler_data = web::Data::new(tx);
    HttpServer::new(move || {
            App::new()
                .app_data(handler_data.clone())
                .service(check)
    })
    .disable_signals()
    .bind(addr)
    .map_err(|_| String::from("actix could not bind"))?
    .run()
    .await
    .map(|_| ())
    .map_err(|_| String::from("actix done goofed"))
}