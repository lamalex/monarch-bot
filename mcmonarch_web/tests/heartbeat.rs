use futures::future::FutureExt;
use mcmonarch_web;
use std::net::TcpListener;

#[actix_rt::test]
async fn heartbeating() {
    let bind_addr = spawn_app();
    let client = reqwest::Client::new();

    let response = client
        .get(&format!("{}/heartbeat", &bind_addr))
        .send()
        .await
        .expect("Failed to execute request");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

fn spawn_app() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");
    let port = listener.local_addr().unwrap().port();
    let verify_cb = Box::new(|_| async { Ok(()) }.boxed());
    let server = mcmonarch_web::run(listener, verify_cb).expect("Failed to launch test server");

    let _ = tokio::spawn(server);

    format!("http://127.0.0.1:{}", port)
}
