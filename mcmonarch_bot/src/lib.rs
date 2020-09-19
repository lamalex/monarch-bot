use color_eyre::eyre;
use log;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serenity::{
    async_trait,
    http::client,
    model::{channel::Message, gateway::Ready, id::GuildId, guild::Member},
    utils::MessageBuilder, utils::EmbedMessageBuilding,
    prelude::*,
};
use simplecrypt;
use std::env;

#[derive(Debug, Serialize, Deserialize)]
struct Email<'a> {
    from: &'static str,
    to: &'a str,
    subject: &'static str,
    template: &'static str,
    #[serde(rename="h:X-Mailgun-Variables")]
    variables: String,
}

impl<'a> Email<'a> {
    pub fn new(to: &'a str, vars: EmailVariables) -> eyre::Result<Self> {
        let vars_ser = serde_json::to_string(&vars).map_err(|e| eyre::eyre!(e))?;
        
        Ok(Self {
            from: "CS @ ODU Discord <postmaster@mg.odu-cs-community.codes>",
            to,
            subject: "Verify your email",
            template: "odu-cs-monarch-verify",
            variables: vars_ser,
        })
    }

    pub async fn send(&self) -> eyre::Result<()> {
        let api_key = env::var("MCMONARCH_MAIL_API_KEY")?;

        reqwest::Client::new()
            .post("https://api.mailgun.net/v3/mg.odu-cs-community.codes/messages")
            .basic_auth("api", Some(api_key))
            .form(self)
            .send()
            .await
            .map(|_| ())
            .map_err( |e| eyre::eyre!(e))
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
}

#[derive(Debug, Serialize, Deserialize)]
struct EmailVariables {
    uid: Vec<u8>,
}

impl EmailVariables {
    pub fn new(data: &str) -> eyre::Result<Self> {
        let passphrase = env::var("MCMONARCH_CIPHER_PASSPHRASE")?;
        let ciphered_uid = simplecrypt::encrypt(data.as_bytes(), passphrase.as_bytes());

        Ok(Self {
            uid: ciphered_uid
        })
    }
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if McmonarchBot::should_ignore(&ctx, &msg).await {
            return;
        }
        
        match Email::extract_odu_email(&msg.content) {
            Some(email_addr) => {
                let _ = msg.react(&ctx, '👍').await;

                // TODO: This is a good place to implement use builder pattern
                let vars = EmailVariables::new(&msg.author.id.to_string()).unwrap();
                let email=  Email::new(&email_addr, vars).unwrap();
                
                match email.send().await {
                    // This isn't doing what we want. we need to check status codes
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
                    _ => {
                        let _ = msg.reply(&ctx, 
                            ":ok_hand: Check your email. I'm sending a verification link to that address."
                        ).await;
                    }
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

pub struct McmonarchBot { }

impl McmonarchBot {
    pub async fn verify(data: Vec<u8>) -> eyre::Result<()> {
        let gid: u64 = 740971495521779795;
        let rid: u64 = 756230811816296480;

        let token = env::var("MCMONARCH_DISCORD_TOKEN")?;
        let passphrase = env::var("MCMONARCH_CIPHER_PASSPHRASE")?;
        let uid = String::from_utf8(simplecrypt::decrypt(
            &data, 
            passphrase.as_bytes())?)
            .map_err(|e| eyre::eyre!(e))?
            .parse::<u64>()?;
        
        let client = client::Http::new_with_token(&token);
        client.add_member_role(gid, uid, rid).await.map_err(|e| eyre::eyre!(e))
    }

    /// Monarchy Mcmonarch Bot ignores 
    ///     1. messages sent outside of a private channel (DMS)
    ///     2. its own messages
    pub async fn should_ignore(cache: impl AsRef<serenity::cache::Cache>, msg: &Message) -> bool {
        !msg.is_private() || msg.is_own(cache).await
    }
}

pub async fn get_bot(token: &str) -> eyre::Result<()> {
    let mut client = Client::new(token)
        .event_handler(Handler)
        .await
        .expect("Err creating client");
    
    client.start().await.map(|_| ()).map_err(|e| eyre::eyre!(e))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn extracts_odu_valid_email() {
        let expected = Some("monarchsrule@odu.edu".to_string());
        
        assert_eq!(expected, Email::extract_odu_email("monarchsrule@odu.edu"));
        assert_eq!(expected, Email::extract_odu_email("my email is monarchsrule@odu.edu"));
        assert_eq!(expected, Email::extract_odu_email("monarchsrule@odu.edu. hmu!! 💝"));
        assert_eq!(expected, Email::extract_odu_email("my email is monarchsrule@odu.edu cash me outside"));
    }

    #[test]
    fn fails_invalid_odu_email() {
        assert_eq!(None, Email::extract_odu_email(""));
        assert_eq!(None, Email::extract_odu_email("i_love_guac@hotmail.com"));
    }

    #[test]
    #[ignore]
    fn send_email() -> eyre::Result<()> {
        let api_key = dotenv::var("MCMONARCH_MAIL_API_KEY")?;
        std::env::set_var("MCMONARCH_MAIL_API_KEY", api_key);
        let email = Email::new("alex.launi@gail.com", EmailVariables::new("dave grohl sucks")?)?;
        tokio_test::block_on(
            email.send()
        )?;

        std::env::set_var("MAIL_API_KEY", "");
        Ok(())
    }

    #[test]
    fn test_encrypt_decrypt() {
        let key = dotenv::var("MCMONARCH_CIPHER_PASSPHRASE").unwrap();
        std::env::set_var("MCMONARCH_CIPHER_PASSPHRASE", &key);
        let plaintext = "alaun001@odu.edu".as_bytes();
        let cipher = simplecrypt::encrypt(plaintext, key.as_bytes());
        assert_eq!(plaintext, simplecrypt::decrypt(&cipher, key.as_bytes()).unwrap());
        std::env::set_var("MCMONARCH_CIPHER_PASSPHRASE", "");
    }
}