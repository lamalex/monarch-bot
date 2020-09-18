
use actix_web::{get, App, Error, HttpResponse, HttpServer};
use bytes::Bytes;
use futures::future::ok;
use futures::stream::once;
use dotenv::dotenv;
use std::env;

use futures::try_join;

use serenity::{
    async_trait,
    model::{channel::Message, gateway::Ready},
    prelude::*,
};

use warp::Filter;
struct Handler;

#[async_trait]
impl EventHandler for Handler {
    // Set a handler for the `message` event - so that whenever a new message
    // is received - the closure (or function) passed will be called.
    //
    // Event handlers are dispatched through a threadpool, and so multiple
    // events can be dispatched simultaneously.
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.content == "!ping" {
            // Sending a message can fail, due to a network error, an
            // authentication error, or lack of permissions to post in the
            // channel, so log to stdout when some error happens, with a
            // description of it.
            if let Err(why) = msg.channel_id.say(&ctx.http, "Pong!").await {
                println!("Error sending message: {:?}", why);
            }
        }
    }

    // Set a handler to be called on the `ready` event. This is called when a
    // shard is booted, and a READY payload is sent by Discord. This payload
    // contains data like the current user's guild Ids, current user data,
    // private channels, and more.
    //
    // In this case, just print what the current user's username is.
    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

#[actix_web::main]
async fn main() {
    dotenv().ok();
    let warp_fut = get_warp();
    let bot_gut = get_bot();
    let actix_fut = get_actix();
    try_join!(warp_fut, bot_gut, actix_fut).unwrap();

}

#[get("/stream")]
async fn stream() -> HttpResponse {
    let body = once(ok::<_, Error>(Bytes::from_static(b"test")));

    HttpResponse::Ok()
        .content_type("application/json")
        .streaming(body)
}

async fn get_actix() -> Result<(), String> {
    HttpServer::new(|| App::new().service(stream))
        .bind("127.0.0.1:8080")
        .map_err(|_| String::from("actix could not bind"))?
        .run()
        .await
        .map(|_| ())
        .map_err(|_| String::from("actix done goofed"))
        
}

async fn get_warp() -> Result<(), String> {
    // GET /hello/warp => 200 OK with body "Hello, warp!"
    let hello = warp::path!("hello" / String)
        .map(|name| format!("Hello, {}!", name));

    warp::serve(hello)
        .run(([127, 0, 0, 1], 3030))
        .await;
    Ok(())
}

async fn get_bot() -> Result<(), String> {
    let token = env::var("DISCORD_TOKEN")
        .expect("Expected a token in the environment");

    // Create a new instance of the Client, logging in as a bot. This will
    // automatically prepend your bot token with "Bot ", which is a requirement
    // by Discord for bot users.
    let mut client = Client::new(&token)
        .event_handler(Handler)
        .await
        .expect("Err creating client");
    
    client.start().await.map(|_| ()).map_err(|_| String::from("bot failed"))
}