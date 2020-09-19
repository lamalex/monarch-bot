
use dotenv::dotenv;
use std::env;
use color_eyre::{eyre, eyre::Result};

use tokio::sync::mpsc;
use futures::try_join;

use mcmonarch_bot;
use mcmonarch_web;

use std::net::SocketAddrV4;

const WEB_IP_ENVVAR: &'static str = "MCMONARCH_WEB_PORT";
const WEB_PORT_ENVVAR: &'static str = "MCMONARCH_WEB_IP";
const BOT_TOKEN_ENVVAR: &'static str = "MCMONARCH_DISCORD_TOKEN";


#[actix_web::main]
pub async fn main() -> Result<()> {
    dotenv().ok();
    pretty_env_logger::init();
    color_eyre::install()?;

    let bot_token = env::var(BOT_TOKEN_ENVVAR)?;
    let web_ip = env::var(WEB_IP_ENVVAR)?;
    let web_port = env::var(WEB_PORT_ENVVAR)?;
    let web_addr = format!("{}:{}", web_port, web_ip)
        .parse::<SocketAddrV4>()?;

    let (tx, rx) = mpsc::unbounded_channel::<String>();

    let bot_fut = mcmonarch_bot::get_bot(&bot_token, rx);
    let web_fut = mcmonarch_web::get_web(web_addr, tx);
    
    try_join!(bot_fut, web_fut)
        .map(|_| ())
        .map_err(|e| eyre::eyre!(e))
}