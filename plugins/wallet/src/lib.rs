use http::cookies::{CookieJar, Key};
use http::{headers, mime, Body, Cookie, Method, Request, Response, StatusCode};
use libwallet::{sr25519, Pair, SimpleVault, Wallet};
use path_tree::PathTree;
use valor::*;
use webauthn_rs::{ephemeral::WebauthnEphemeralConfig, Webauthn};

const TEST_ACCOUNT: &str = "0x329e7f8361cd64c7a60f4e75cd273a52c42930aa9d26955e3e7111eb4136432c";

type KeyPair = sr25519::Pair;
type VWallet = Wallet<SimpleVault<KeyPair>>;
type Router = PathTree<Cmd>;
type Result<T> = std::result::Result<T, Error>;

enum Cmd {
    Demo,
    Register,
    Sign,
    Unlock,
}

#[vlugin]
pub async fn on_create(cx: &mut Context) {
    let mut wallet: VWallet = cx
        .config::<&str>()
        .unwrap_or(TEST_ACCOUNT)
        .parse::<SimpleVault<_>>()
        .expect("root seed")
        .into();
    wallet.unlock("").await.expect("root wallet");
    cx.set(wallet);

    cx.set({
        let mut p = Router::new();
        p.insert("/", Cmd::Demo);
        p.insert("/register", Cmd::Register);
        p.insert("/unlock", Cmd::Unlock);
        p.insert("/sign", Cmd::Sign);
        p
    });
}

/// Wallet plugin
///
/// `GET	/open` Use it to get a challenge that must be signed by a known private key
/// `POST	/open` Provide credentials to set an encrypted cookie that unlocks the user wallet
/// `POST	/sign` Sign a binary payload with active user's key
pub async fn on_request(cx: &Context, mut req: Request) -> http::Result<Response> {
    let routes = cx.get::<Router>();
    let url = req.url();
    let (action, _params) = routes
        .find(url.path())
        .ok_or_else(|| http::Error::from_str(StatusCode::NotFound, ""))?;
    let wallet = cx.get::<VWallet>();

    let res = match (req.method(), action) {
        (Method::Get, Cmd::Demo) => {
            let mut res: Response = include_bytes!("demo.html")[..].into();
            res.append_header(headers::CONTENT_TYPE, mime::HTML);
            Ok(res)
        }
        (Method::Get, Cmd::Register) => send_reg_challenge(&mut req, &mut new_webauthn()).await,
        (Method::Post, Cmd::Register) => register_user(&mut req).await,
        (Method::Get, Cmd::Unlock) => send_auth_challenge(&mut req, &mut new_webauthn()).await,
        (Method::Post, Cmd::Unlock) => unlock_user_wallet(&mut req).await,
        (Method::Post, Cmd::Sign) => sign_payload(&mut req, &wallet).await,
        _ => Ok(StatusCode::MethodNotAllowed.into()),
    }?;
    Ok(res)
}

const USER_WALLET: &str = "wallet";

/// Validates a logged in user, retreives her wallet and signs an arbitrary binary payload
async fn sign_payload(req: &mut Request, wallet: &VWallet) -> Result<Response> {
    let root_account = wallet.root_account().map_err(|_| Error::RootWalletLocked)?;
    // reusing wallet's key to encrypt/decrypt cookies
    let mut cookies = request_cookies(req)?;
    let cookies = cookies.private(&Key::derive_from(&root_account.to_raw_vec()));

    // Generate user wallet account from the session
    let user_wallet = cookies.get(USER_WALLET).ok_or(Error::Unknown)?;
    let account: KeyPair = root_account
        .derive(vec![user_wallet.value().into()].into_iter(), None)
        .map_err(|_| Error::Unknown)?
        .0;

    let payload = req.body_bytes().await.map_err(|_| Error::Unknown)?;
    let signed_payload = account.sign(&payload);
    let bytes: &[u8] = signed_payload.as_ref();
    Ok(bytes.into())
}

async fn register_user(_req: &mut Request) -> Result<Response> {
    todo!()
}

type WebAuthn = Webauthn<WebauthnEphemeralConfig>;
fn new_webauthn() -> WebAuthn {
    Webauthn::new(WebauthnEphemeralConfig::new(
        "vln",
        "api.valiu.dev",
        "123",
        None,
    ))
}

/// TODO store registration state
/// Send a WebAuthn challenge for passwordless registration
async fn send_reg_challenge(req: &Request, wan: &mut WebAuthn) -> Result<Response> {
    const USER_PARAM: &str = "u";
    let name = req
        .url()
        .query_pairs()
        .find(|(q, _)| q == USER_PARAM)
        .ok_or(Error::MissingParamerter("name".into()))?
        .1;
    let (challenge, _) = wan
        .generate_challenge_register(&name.to_string(), None)
        .map_err(|_| Error::Unknown)?;

    let mut res: Response = Body::from_json(&challenge)?.into();
    res.append_header(headers::CONTENT_TYPE, mime::JSON);
    Ok(res)
}

/// TODO store authentication state
/// Send a WebAuthn challenge for passwordless login
async fn send_auth_challenge(req: &Request, wan: &mut WebAuthn) -> Result<Response> {
    const USER_PARAM: &str = "u";
    let _name = req
        .url()
        .query_pairs()
        .find(|(q, _)| q == USER_PARAM)
        .ok_or(Error::MissingParamerter("name".into()))?
        .1;
    let (challenge, _) = wan
        .generate_challenge_authenticate(vec![], None)
        .map_err(|_| Error::Unknown)?;

    let mut res: Response = Body::from_json(&challenge)?.into();
    res.append_header(headers::CONTENT_TYPE, mime::JSON);
    Ok(res)
}

async fn unlock_user_wallet(_req: &mut Request) -> Result<Response> {
    todo!()
}

// build a cookie jar from a request used to access individual cookies
fn request_cookies(req: &Request) -> Result<CookieJar> {
    let cookie_header = req
        .header(&headers::COOKIE)
        .map(|h| h.get(0))
        .flatten()
        .ok_or(Error::WalletClosed)?;
    let mut jar = CookieJar::new();
    for pair in cookie_header.as_str().split(';') {
        if let Ok(cookie) = Cookie::parse_encoded(String::from(pair)) {
            jar.add_original(cookie);
        }
    }
    Ok(jar)
}

pub enum Error {
    Unknown,
    RootWalletLocked,
    WalletClosed,
    MissingParamerter(String),
    Http(http::Error),
}

impl From<http::Error> for Error {
    fn from(err: http::Error) -> Self {
        Error::Http(err)
    }
}

impl From<Error> for http::Error {
    fn from(e: Error) -> Self {
        match e {
            Error::RootWalletLocked => {
                Self::from_str(StatusCode::ServiceUnavailable, "Bad configuration")
            }
            Error::WalletClosed => Self::from_str(StatusCode::Unauthorized, "Unlock wallet"),
            Error::Http(err) => err,
            Error::MissingParamerter(_) => Self::from_str(StatusCode::BadRequest, "Foo"),
            _ => Self::from_str(StatusCode::InternalServerError, ""),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[async_std::test]
    async fn signs_request_payload() {
        let mut cx = Context::default();
        on_create(&mut cx).await;

        let w = cx.get::<VWallet>();
        let root = w.root_account().expect("unlocked");
        let user_wallet = "//foo";

        let mut req = Request::new(Method::Post, "foo:///sign");
        req.append_header(
            headers::COOKIE,
            test_cookies(&root, user_wallet)
                .get(USER_WALLET)
                .unwrap()
                .encoded()
                .to_string(),
        );
        let message = &b"message"[..];
        req.set_body(message);

        let mut res = on_request(&cx, req).await.unwrap();
        assert_eq!(res.status(), StatusCode::Ok);

        let body = res.body_bytes().await.expect("response");
        let user_wallet = root
            .derive(vec![user_wallet.into()].into_iter(), None)
            .unwrap()
            .0;
        assert!(KeyPair::verify_weak(&body, message, &user_wallet.public()));
    }

    fn test_cookies(root: &KeyPair, user: &'static str) -> CookieJar {
        let mut jar = CookieJar::new();
        let mut private = jar.private(&Key::derive_from(&root.to_raw_vec()));
        private.add_original(Cookie::new(USER_WALLET, user));
        jar
    }
}
