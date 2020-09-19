use color_eyre::eyre;
use log;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serenity::{
    async_trait,
    model::{channel::Message, gateway::Ready, id::GuildId, guild::Member},
    utils::MessageBuilder, utils::EmbedMessageBuilding,
    prelude::*,
};
use simplecrypt;
use std::env;
use futures::join;
use tokio::sync::mpsc;

#[derive(Serialize, Deserialize)]
struct MailApi<'a> {
    from: &'static str,
    to: &'a str,
    subject: &'static str,
    template: &'static str,
    #[serde(rename="h:X-Mailgun-Variables")]
    variables: &'a str,
}

#[derive(Serialize, Deserialize)]
struct MailgunVariables {
    ciphertext: Vec<u8>
}

// user portion of regular expression courtesy of https://html.spec.whatwg.org/multipage/input.html#valid-e-mail-address
fn extract_odu_email(msg: &str) -> Option<String> {
    let re = Regex::new(r"[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+@odu\.edu").unwrap();
    re.captures_iter(msg)
    .nth(0)
    .map(|cap| 
        cap.get(0)
        .map(|cap| String::from(cap.as_str()))
    ).flatten()
}

async fn send_email(addr: &str) -> eyre::Result<()> {
    let passphrase = env::var("MCMONARCH_CIPHER_PASSPHRASE")?;

    let ciphered_addr = simplecrypt::encrypt(addr.as_bytes(), passphrase.as_bytes());
    let payload_json = serde_json::to_string(&MailgunVariables {
        ciphertext: ciphered_addr
    })?;

    let mail_params = MailApi {
        from: "Mailgun Sandbox <postmaster@sandboxf623066ae65145e8b08901e5539d90c6.mailgun.org>",
        to: addr,
        subject: "CS @ ODU Discord -- verify your email",
        template: "odu-cs-monarch-verify",
        variables: &payload_json,
    };

    let api_key = env::var("MCMONARCH_MAIL_API_KEY")?;

    reqwest::Client::new()
        .post("https://api.mailgun.net/v3/sandboxf623066ae65145e8b08901e5539d90c6.mailgun.org/messages")
        .basic_auth("api", Some(api_key))
        .form(&mail_params)
        .send()
        .await
        .map(|_| ())
        .map_err( |e| eyre::eyre!(e))
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    // Set a handler for the `message` event - so that whenever a new message
    // is received - the closure (or function) passed will be called.
    //
    // Event handlers are dispatched through a threadpool, and so multiple
    // events can be dispatched simultaneously.
    async fn message(&self, ctx: Context, msg: Message) {
        if !msg.is_private() || msg.is_own(&ctx).await {
            return;
        }

        match extract_odu_email(&msg.content) {
            Some(email_addr) => {
                let react = msg.react(&ctx, '👍');
                let reply = msg.reply(&ctx, 
                    ":ok_hand: Check your email. I'm sending a verification link to that address."
                );

                let _ = join!(react, reply);

                match send_email(&email_addr).await {
                    Err(e) => {
                        let reply = MessageBuilder::new()
                            .push_line(":fire_engine: Uh-oh \n \
                                Something went wrong sending your verification email. \n \
                                Try again later maybe? If this keeps happening file an issue on gitlab.")
                            .push_named_link(
                                "CS @ ODU Meta gitlab", 
                                "https://git-community.cs.odu.edu/community-discord/meta/-/issues/new"
                            )
                            .build();

                        let _ = msg.reply(&ctx, reply).await;
                        log::warn!("{}", e);
                    }
                    _ => {}
                };  
            }
            None => {
                let _ = msg.reply(&ctx, format!(
                    ":confused: \n \
                    {} doesn't look like an @odu.edu email to me.\n \
                    Can I please have your @odu.edu email address? :pray:",
                    msg.content
                )).await;
            }
        }
    }

    async fn guild_member_addition(&self, ctx: Context, _guild_id: GuildId, new_member: Member) {
        log::info!("{:?} (id: {:?} joined with roles: {:?}.", new_member.display_name(), new_member.user.id, new_member.roles);
        let welcome_msg = "\n \
            Welcome to the CS @ ODU discord channel. \n \
            Please make sure you review the #rules. This channel is for ODU students only. \n \
            Could I have your @odu email address and we can get you verified?";

        // REMOVE: for development only
        if new_member.display_name().into_owned() != String::from("imsorrybutthisisatestacct") {
            return 
        }

        let _ = new_member.user.direct_message(&ctx, |m| {
            let msg = MessageBuilder::new()
                .push(":wave: ")
                .mention(&new_member.user)
                .push(welcome_msg)
                .build();
            m.content(msg)
        })
        .await
        .map(|_| ())
        .map_err(|e| log::error!("DM error: {:?}", e));
    }

    // Set a handler to be called on the `ready` event. This is called when a
    // shard is booted, and a READY payload is sent by Discord. This payload
    // contains data like the current user's guild Ids, current user data,
    // private channels, and more.
    //
    // In this case, just print what the current user's username is.
    async fn ready(&self, _: Context, ready: Ready) {
        log::info!("{} is connected!", ready.user.name);
    }
}

pub async fn get_bot(token: &str, rx: mpsc::UnboundedReceiver<String>) -> Result<(), String> {
    let mut client = Client::new(token)
        .event_handler(Handler)
        .await
        .expect("Err creating client");

    client.start().await.map(|_| ()).map_err(|_| String::from("bot failed"))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn extracts_odu_valid_email() {
        let expected = Some("monarchsrule@odu.edu".to_string());
        
        assert_eq!(expected, extract_odu_email("monarchsrule@odu.edu"));
        assert_eq!(expected, extract_odu_email("my email is monarchsrule@odu.edu"));
        assert_eq!(expected, extract_odu_email("monarchsrule@odu.edu. hmu!! 💝"));
        assert_eq!(expected, extract_odu_email("my email is monarchsrule@odu.edu cash me outside"));
    }

    #[test]
    fn fails_invalid_odu_email() {
        assert_eq!(None, extract_odu_email(""));
        assert_eq!(None, extract_odu_email("i_love_guac@hotmail.com"));
    }

    #[test]
    #[ignore]
    fn send_email() -> eyre::Result<()> {
        let api_key = dotenv::var("MCMONARCH_MAIL_API_KEY")?;
        std::env::set_var("MAIL_API_KEY", api_key);
        tokio_test::block_on(
            super::send_email("alex.launi@gmail.com")
        )?;

        std::env::set_var("MAIL_API_KEY", "");
        Ok(())
    }
}