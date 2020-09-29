use color_eyre::eyre;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Serialize, Deserialize)]
pub struct Email {
    from: &'static str,
    to: String,
    subject: &'static str,
    template: &'static str,
    #[serde(rename = "h:X-Mailgun-Variables")]
    variables: String,
}

impl Email {
    pub fn new(to: &str, vars: EmailVariables) -> eyre::Result<Self> {
        let vars_ser = serde_json::to_string(&vars).map_err(|e| eyre::eyre!(e))?;
        
        let to = match Self::extract_odu_email(to) {
            Some(email) => Ok(email),
            None => Err(eyre::eyre!("{} was not a valid ODU email", to))
        }?.to_owned();

        Ok(Self {
            from: "CS @ ODU Discord <postmaster@mg.odu-cs-community.codes>",
            to,
            subject: "Verify your email",
            template: "odu-cs-monarch-verify",
            variables: vars_ser,
        })
    }

    pub async fn send(&self) -> eyre::Result<reqwest::Response> {
        let api_key = env::var("MCMONARCH_MAIL_API_KEY")?;

        reqwest::Client::new()
            .post("https://api.mailgun.net/v3/mg.odu-cs-community.codes/messages")
            .basic_auth("api", Some(api_key))
            .form(self)
            .send()
            .await
            .map_err(|e| eyre::eyre!(e))
    }

    // user portion of regular expression courtesy of https://html.spec.whatwg.org/multipage/input.html#valid-e-mail-address
    pub fn extract_odu_email(msg: &str) -> Option<&str> {
        let re = Regex::new(r"[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+@odu\.edu").unwrap();
        re.captures_iter(msg)
            .next()
            .map(|cap| cap.get(0).map(|cap| cap.as_str()))
            .flatten()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EmailVariables {
    uid: Vec<u8>,
}

impl EmailVariables {
    pub fn new(data: &str) -> eyre::Result<Self> {
        let passphrase = env::var("MCMONARCH_CIPHER_PASSPHRASE")?;
        let ciphered_uid = simplecrypt::encrypt(data.as_bytes(), passphrase.as_bytes());

        Ok(Self { uid: ciphered_uid })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_odu_valid_email() {
        let expected = Some("monarchsrule@odu.edu");

        assert_eq!(expected, Email::extract_odu_email("monarchsrule@odu.edu"));
        assert_eq!(
            expected,
            Email::extract_odu_email("my email is monarchsrule@odu.edu")
        );
        assert_eq!(
            expected,
            Email::extract_odu_email("monarchsrule@odu.edu. hmu!! 💝")
        );
        assert_eq!(
            expected,
            Email::extract_odu_email("my email is monarchsrule@odu.edu cash me outside")
        );
    }

    #[test]
    fn fails_invalid_odu_email() {
        assert_eq!(None, Email::extract_odu_email(""));
        assert_eq!(None, Email::extract_odu_email("i_love_guac@hotmail.com"));
        assert_eq!(None, Email::extract_odu_email("rustrules"));
        assert_eq!(None, Email::extract_odu_email("rustrules@"));
        assert_eq!(None, Email::extract_odu_email("rustrules@odu"));
        assert_eq!(None, Email::extract_odu_email("rustrules@odu."));
        assert_eq!(None, Email::extract_odu_email("rustrules@odu.ed"));
    }

    #[test]
    #[ignore]
    fn send_email() -> eyre::Result<()> {
        let api_key = dotenv::var("MCMONARCH_MAIL_API_KEY")?;
        std::env::set_var("MCMONARCH_MAIL_API_KEY", api_key);
        let email = Email::new(
            "alex.launi@gail.com",
            EmailVariables::new("dave grohl sucks")?,
        )?;
        tokio_test::block_on(email.send())?;

        std::env::set_var("MAIL_API_KEY", "");
        Ok(())
    }
}