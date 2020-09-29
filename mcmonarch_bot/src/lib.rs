use color_eyre::eyre;
use serenity::{
    async_trait,
    http::client,
    model::{channel::Message, gateway::Ready, guild::Member, id::GuildId},
    prelude::*,
    utils::EmbedMessageBuilding,
    utils::MessageBuilder,
};
use std::env;

mod mailer;
use mailer::{Email, EmailVariables};

#[async_trait]
impl EventHandler for McmonarchBot {
    async fn message(&self, ctx: Context, msg: Message) {
        if McmonarchBot::should_ignore(&ctx, &msg).await {
            return;
        }

        /*
        What I want this to look like
        let email = Email::builder()
            .to(Email::extract_odu_email(&msg.content))
            .with_var("uid", &msg.author.id.to_string())
            .build();
        
        match email {
            Ok(email) => ...,
            Err(InvalidEmailAddr) => ...,
            Err(_) => ...,
        }
        */

        match Email::extract_odu_email(&msg.content) {
            Some(email_addr) => {
                let _ = msg.react(&ctx, '👍').await;

                // TODO: This is a good place to implement use builder pattern
                let vars = EmailVariables::new(&msg.author.id.to_string()).unwrap();
                let email = Email::new(&email_addr, vars).unwrap();

                match email.send().await {
                    Ok(res) if res.status() == reqwest::StatusCode::OK => {
                        let _ = msg
                        .reply(
                            &ctx,
                            ":ok_hand: \
                        Check your email. I'm sending a verification link to that address. \n \
                        (check your spam if it doesn't show up)",
                        )
                        .await;
                    },
                    Ok(res) => {
                        self.reply_on_email_problem(&ctx, msg).await;
                        log::warn!("{}", res.status());
                    }
                    // This isn't doing what we want. we need to check status codes
                    Err(e) => {
                        self.reply_on_email_problem(&ctx, msg).await;
                        log::warn!("{}", e);
                    }
                };
            }
            None => {
                let _ = msg
                    .reply(
                        &ctx,
                        format!(
                            ":confused: \n \
                    {} doesn't look like an @odu.edu email to me.\n \
                    Can I please have your @odu.edu email address? :pray:",
                            msg.content
                        ),
                    )
                    .await;
            }
        }
    }

    async fn guild_member_addition(&self, ctx: Context, _guild_id: GuildId, new_member: Member) {
        log::info!(
            "{:?} (id: {:?} joined with roles: {:?}.",
            new_member.display_name(),
            new_member.user.id,
            new_member.roles
        );
        let welcome_msg = "\n \
            Welcome to the CS @ ODU discord channel. \n \
            Please make sure you review the #rules. This channel is for ODU students only. \n \
            Could I have your @odu email address and we can get you verified?";

        let _ = new_member
            .user
            .direct_message(&ctx, |m| {
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

pub struct McmonarchBot;

impl McmonarchBot {
    pub async fn verify(data: Vec<u8>) -> eyre::Result<()> {
        let gid: u64 = 740971495521779795;
        let rid: u64 = 756230811816296480;

        let token = env::var("MCMONARCH_DISCORD_TOKEN")?;
        let passphrase = env::var("MCMONARCH_CIPHER_PASSPHRASE")?;
        let uid = String::from_utf8(simplecrypt::decrypt(&data, passphrase.as_bytes())?)
            .map_err(|e| eyre::eyre!(e))?
            .parse::<u64>()?;

        let client = client::Http::new_with_token(&token);
        client
            .add_member_role(gid, uid, rid)
            .await
            .map_err(|e| eyre::eyre!(e))
    }

    /// Monarchy Mcmonarch Bot ignores
    ///     1. messages sent outside of a private channel (DMS)
    ///     2. its own messages
    pub async fn should_ignore(cache: impl AsRef<serenity::cache::Cache>, msg: &Message) -> bool {
        !msg.is_private() || msg.is_own(cache).await
    }

    async fn reply_on_email_problem(&self, ctx: &Context, msg: Message) {
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
    }
}

pub async fn get_bot(token: &str) -> eyre::Result<()> {
    let mut client = Client::new(token)
        .event_handler(McmonarchBot)
        .await
        .expect("Err creating client");

    client.start().await.map(|_| ()).map_err(|e| eyre::eyre!(e))
}
