use actix_files::NamedFile;
use actix_http::error;
use actix_web::{dev::Server, get, web, App, HttpResponse, HttpServer, Result};
use color_eyre::eyre;
use futures::future::BoxFuture;
use std::{net::TcpListener, path::PathBuf};

type VerificationCallback =
    Box<dyn Sync + Send + Fn(Vec<u8>) -> BoxFuture<'static, eyre::Result<()>>>;

/// Handles route /verify/String where string is of them form B,B,B,B,B,...
/// Where B,B,B,... is a comma delimited list of bytes representing the encrypted
/// form of a Discord userid.
/// Turns the comma separated list into a Vec<u8> and passes it to the callback
/// which returns a future stored in data.cb
#[get("/verify/{payload}")]
async fn check(
    payload: web::Path<String>,
    data: web::Data<VerificationCallback>,
) -> actix_web::Result<NamedFile> {
    let inner = payload.into_inner();
    let verify = data.get_ref();
    let path = inner_check(&inner, verify)
        .await
        .map_err(error::ErrorInternalServerError)?;

    Ok(NamedFile::open(path)?)
}

async fn inner_check(data: &str, verify: &VerificationCallback) -> eyre::Result<PathBuf> {
    let bytes = delimited_string_try_into_vec(data, ",")?;

    Ok(match verify(bytes).await {
        Ok(_) => PathBuf::from("mcmonarch_web/static/verified.html"),
        Err(_) => PathBuf::from("mcmonarch_web/static/failed.html"),
    })
}

/// Converts a non-empty string of FromStrs delimited by `delimiter` into a Vec<T>.
/// Gives an error if any fail to parse.
/// Strips empty terminal element if string ends with `delimiter`.
/// Errors on empty input, or if input is not parsable to T
fn delimited_string_try_into_vec<T>(input: &str, delimiter: &str) -> eyre::Result<Vec<T>>
where
    T: std::str::FromStr,
{
    if input.is_empty() {
        return Err(eyre::eyre!("input must have at least 1 element"));
    }

    input
        .split_terminator(delimiter)
        .map(|v| {
            v.parse::<T>()
                .map_err(|_| eyre::eyre!(format!("Failed parsing {}", v)))
        })
        .collect::<Result<Vec<T>, _>>()
}

/// Web server health check API call for remote monitoring
async fn heartbeat() -> HttpResponse {
    HttpResponse::Ok().finish()
}

/// Returns a future that returns a Result<()> for the running webserver.
pub async fn get_web(listener: TcpListener, verify_cb: VerificationCallback) -> eyre::Result<()> {
    run(listener, verify_cb)?
        .await
        .map(|_| ())
        .map_err(|e| eyre::eyre!(e))
}

pub fn run(listener: TcpListener, verify_cb: VerificationCallback) -> eyre::Result<Server> {
    let async_data = web::Data::new(verify_cb);

    let server = HttpServer::new(move || {
        App::new()
            .app_data(async_data.clone())
            .service(check)
            .route("/heartbeat", web::get().to(heartbeat))
    })
    .disable_signals()
    .listen(listener)
    .map_err(|e| eyre::eyre!(e))?
    .run();

    Ok(server)
}

#[cfg(test)]
mod test {
    use super::*;
    use futures::future::FutureExt;

    #[test]
    fn convert_cds_to_vec() {
        let expected = "Hi my name is dingus";
        let input = make_valid_u8_vec_string(expected);
        assert_eq!(
            Vec::<u8>::from(expected),
            delimited_string_try_into_vec::<u8>(&input, ",").unwrap()
        );
    }

    #[test]
    fn fail_convert_cds_to_vec_empty_parse() {
        let start: Vec<u8> = "me amo es dingus".into();
        let input: String = start.iter().map(|&b| format!(",{},", b)).collect();
        assert!(delimited_string_try_into_vec::<u8>(&input, ",").is_err());
    }

    #[test]
    fn convert_cds_rejects_empty() {
        let input = make_valid_u8_vec_string("");
        assert!(delimited_string_try_into_vec::<u8>(&input, ",").is_err());
    }

    #[test]
    fn check_calls_verify_with_expected() {
        async fn test_verify(data: Vec<u8>) -> eyre::Result<()> {
            Ok(assert_eq!(Vec::<u8>::from("je m'appele dingus"), data))
        }

        let input = make_valid_u8_vec_string("je m'appele dingus");
        let cb: VerificationCallback = Box::new(|data| test_verify(data).boxed());

        assert!(tokio_test::block_on(inner_check(&input, &cb)).is_ok());
    }

    fn make_valid_u8_vec_string(input: &str) -> String {
        Vec::<u8>::from(input)
            .iter()
            .map(|b| format!("{},", b))
            .collect()
    }
}
