use color_eyre::{eyre, eyre::Result};
use dotenv::dotenv;
use futures::try_join;
use futures::future::FutureExt;
use std::{env, net::SocketAddrV4};

use mcmonarch_bot;
use mcmonarch_web;

const WEB_IP_ENVVAR: &'static str = "MCMONARCH_WEB_IP";
const WEB_PORT_ENVVAR: &'static str = "PORT";
const BOT_TOKEN_ENVVAR: &'static str = "MCMONARCH_DISCORD_TOKEN";

#[actix_web::main]
pub async fn main() -> Result<()> {
    dotenv().ok();
    pretty_env_logger::init();
    color_eyre::install()?;

    
    let bot_token = env::var(BOT_TOKEN_ENVVAR)?;
    let web_ip = env::var(WEB_IP_ENVVAR)?;
    let web_port = env::var(WEB_PORT_ENVVAR)?;
    println!("FOR DEBUGGING {}:{}", web_ip, web_port);
    let web_addr = format!("{}:{}", web_ip, web_port)
        .parse::<SocketAddrV4>()?;

    let verify_box = Box::new(|data| mcmonarch_bot::McmonarchBot::verify(data).boxed()); 
    let bot_fut = mcmonarch_bot::get_bot(&bot_token);
    let web_fut = mcmonarch_web::get_web(web_addr,  verify_box);
    
    try_join!(bot_fut, web_fut)
        .map(|_| ())
        .map_err(|e| eyre::eyre!(e))
}
