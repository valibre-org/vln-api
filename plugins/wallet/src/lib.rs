use http::cookies::{CookieJar, Key};
use http::{headers, Cookie};
use libwallet::{sr25519, Pair, SimpleVault, Wallet};
use once_cell::sync::Lazy;
use path_tree::PathTree;
use std::env;
use valor::*;

const TEST_ACCOUNT: &str = "0x329e7f8361cd64c7a60f4e75cd273a52c42930aa9d26955e3e7111eb4136432c";
static SEED: Lazy<String> = Lazy::new(|| env::var("WALLET_SEED").unwrap_or(TEST_ACCOUNT.into()));

type KeyPair = sr25519::Pair;
type VWallet = Wallet<SimpleVault<KeyPair>>;

// NOTE initially we'll use a single root account and derive user wallets from it
async fn get_wallet() -> VWallet {
    let mut w: VWallet = SEED.parse::<SimpleVault<_>>().expect("bad seed").into();
    let _ = w.unlock("").await;
    w
}

enum Cmd {
    Open,
    Sign,
}

type Result<T> = std::result::Result<T, Error>;

/// Wallet plugin
///
/// `GET	/open` Use it to get a challenge that must be signed by a known private key
/// `POST	/open` Provide credentials to set an encrypted cookie that unlocks the user wallet
/// `POST	/sign` Sign a binary payload with active user's key
#[vlugin]
async fn wallet_handler(mut req: Request) -> Response {
    let routes = {
        let mut p = PathTree::new();
        p.insert("/open", Cmd::Open);
        p.insert("/sign", Cmd::Sign);
        p
    };
    let url = req.url();
    let action = routes.find(url.path());
    if action.is_none() {
        return StatusCode::NotFound.into();
    }
    let (action, _params) = action.unwrap();

    let wallet = get_wallet().await;

    match (req.method(), action) {
        (Method::Get, Cmd::Open) => send_challenge().await,
        (Method::Post, Cmd::Open) => open_wallet().await,
        (Method::Post, Cmd::Sign) => sign_payload(&mut req, &wallet).await,
        _ => Ok(StatusCode::MethodNotAllowed.into()),
    }
    .unwrap_or_else(Into::into)
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

async fn send_challenge() -> Result<Response> {
    todo!()
}

async fn open_wallet() -> Result<Response> {
    todo!()
}

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
}

impl From<Error> for Response {
    fn from(e: Error) -> Self {
        match e {
            Error::RootWalletLocked => StatusCode::ServiceUnavailable.into(),
            Error::WalletClosed => StatusCode::Unauthorized.into(),
            _ => StatusCode::InternalServerError.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[async_std::test]
    async fn signs_request_payload() {
        let w = get_wallet().await;
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

        let mut res = wallet_handler(req).await;
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
