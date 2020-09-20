use actix_files::NamedFile;
use actix_http::error;
use actix_web::{get, web, App, Result, HttpServer};
use color_eyre::eyre;
use futures::future::BoxFuture;
use std::{path::PathBuf, net::ToSocketAddrs};

type VerificationCallback = Box<dyn Sync + Send + Fn(Vec<u8>) -> BoxFuture<'static, eyre::Result<()>>>;

/// Handles route /verify/String where string is of them form B,B,B,B,B,...
/// Where B,B,B,... is a comma delimited list of bytes representing the encrypted
/// form of a Discord userid.
/// Turns the comma separated list into a Vec<u8> and passes it to the callback
/// which returns a future stored in data.cb
#[get("/verify/{payload}")]
async fn check(payload: web::Path<String>, data: web::Data<VerificationCallback>) -> actix_web::Result<NamedFile> 
{
    let inner = payload.into_inner();
    
    let bytes = delimited_string_try_into_vec(&inner, ",")
        .map_err(|e| error::ErrorInternalServerError(e))?;

    let verify = &data.as_ref();
    let path =  match verify(bytes).await {
        Ok(_) => PathBuf::from("mcmonarch_web/static/verified.html"),
        Err(_) => PathBuf::from("mcmonarch_web/static/failed.html")
    };
    
    Ok(NamedFile::open(path)?)
}

/// Converts a string of FromStrs delimited by `delimiter` into a Vec<T>.
/// Gives an error if any fail to parse. 
/// Strips empty terminal element if string ends with `delimiter`.
fn delimited_string_try_into_vec<T>(input: &str, delimiter: &str) -> eyre::Result<Vec<T>> where T: std::str::FromStr {
    input.split_terminator(delimiter)
        .map(|v| v.parse::<T>().map_err(|_| eyre::eyre!(format!("Failed parsing {}", v))))
        .collect::<Result<Vec<T>, _>>()
}

/// Returns a future that returns a Result<()> for the running webserver.
pub async fn get_web(addr: impl ToSocketAddrs, verify_cb: VerificationCallback) -> eyre::Result<()> 
{
    let async_data = web::Data::new(verify_cb);
    HttpServer::new(move || {
            App::new()
                .app_data(async_data.clone())
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

#[cfg(test)]
    mod test {
    use super::*;

    #[test]
    fn convert_cds_to_vec() {
        let expected: Vec<u8> = "Hi my name is Dingus".into();
        let input: String = expected.iter().map(|&b| format!("{},", b)).collect();
        assert_eq!(expected, delimited_string_try_into_vec::<u8>(&input, ",").unwrap());
    }

    #[test]
    fn fail_convert_cds_to_vec_empty_parse() {
        let start: Vec<u8> = "me amo es dingus".into();
        let input: String = start.iter().map(|&b| format!(",{},", b)).collect();
        assert!(delimited_string_try_into_vec::<u8>(&input, ",").is_err());
    }
}   