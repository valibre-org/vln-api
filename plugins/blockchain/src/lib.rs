mod util_hash;
mod util_meta;
mod util_txid;

use frame_metadata::{RuntimeMetadata, RuntimeMetadataPrefixed, StorageEntryType, StorageHasher};
use http::{content::Accept, mime, Mime};
use jsonrpc::serde_json::{to_string, value::RawValue};
use once_cell::sync::OnceCell;
use parity_scale_codec::Decode;
use path_tree::PathTree;
use std::borrow::Cow;
use util_hash::hash;
use util_meta::MetaExt;
use valor::*;

const NODE_ENDPOINT: &str = "http://vln.valiu";
const SCALE_MIME: &str = "application/scale";
const BASE58_MIME: &str = "application/base58";

static METADATA: OnceCell<RuntimeMetadata> = OnceCell::new();

enum Cmd {
    Meta,
    Storage,
}

type Result<T> = std::result::Result<T, Error>;

#[vlugin]
async fn blockchain_handler(req: Request) -> Response {
    let routes = {
        let mut p = PathTree::new();
        p.insert("/meta", Cmd::Meta);
        p.insert("/:module/:item", Cmd::Storage);
        p
    };

    let url = req.url();
    let action = routes.find(url.path());
    if action.is_none() {
        return StatusCode::NotFound.into();
    }
    let (action, params) = action.unwrap();

    // Use content negotiation to determine the response type
    // By default return data in SCALE encoded binary format
    let response_type = Accept::from_headers(&req)
        .expect("Valid Accept header")
        .unwrap_or_else(Accept::new)
        .negotiate(&[
            mime::PLAIN.essence().into(),
            SCALE_MIME.into(),
            BASE58_MIME.into(),
        ])
        .map(|c| c.value().as_str().into())
        .unwrap_or_else(|_| SCALE_MIME.into());

    match (req.method(), action) {
        (Method::Get, Cmd::Meta) => get_meta(&response_type).await,
        (Method::Get, Cmd::Storage) => {
            #[inline]
            fn query_key<'a>(url: &'a Url, name: &str) -> Option<Cow<'a, str>> {
                url.query_pairs().find(|k| k.0 == name).map(|k| k.1)
            }
            get_storage(
                &response_type,
                params[0].1,
                params[1].1,
                query_key(url, "k"),
                query_key(url, "k2"),
            )
            .await
        }
        _ => Ok(StatusCode::MethodNotAllowed.into()),
    }
    .unwrap_or_else(Into::into)
}

/// GET the SCALE encoded metadata of the blockchain node
async fn get_meta(mime: &Mime) -> Result<Response> {
    rpc("state_getMetadata", &[])
        .await
        .map(|r| response_from_type(mime, r))
}

async fn get_decoded_meta() -> Result<&'static RuntimeMetadata> {
    let meta = METADATA.get();
    if meta.is_some() {
        return meta.ok_or(Error::Unknown);
    }

    let meta = get_meta(&SCALE_MIME.into())
        .await?
        .body_bytes()
        .await
        .map_err(|_| Error::Unknown)
        .and_then(|m| {
            RuntimeMetadataPrefixed::decode(&mut &*m)
                .map(|m| m.1)
                .map_err(|e| Error::Decode(e.to_string()))
        })?;
    Ok(METADATA.get_or_init(|| meta))
}

/// Query a storage value of the node
async fn get_storage(
    mime: &Mime,
    module: &str,
    name: &str,
    k1: Option<Cow<'_, str>>,
    k2: Option<Cow<'_, str>>,
) -> Result<Response> {
    let meta = get_decoded_meta().await?;
    let module_name = to_camel(&module.to_lowercase());
    let entry = meta.entry(&module_name, &to_camel(&name.to_lowercase()));
    if entry.is_none() {
        return Ok(StatusCode::NotFound.into());
    }
    let entry = entry.unwrap();

    // Storage keys are prefixed with the module name + storage item
    let mut key = hash(&StorageHasher::Twox128, &module_name);
    key.push_str(&hash(&StorageHasher::Twox128, &entry.name.to_string()));

    let key = format!(
        "\"0x{}\"",
        match entry.ty {
            StorageEntryType::Plain(_) => key,
            StorageEntryType::Map { ref hasher, .. } => {
                if k1.is_none() || k1.as_ref().unwrap().is_empty() {
                    return Ok(StatusCode::BadRequest.into());
                }
                key.push_str(&hash(hasher, &k1.unwrap()));
                println!("> {}", key);
                key
            }
            StorageEntryType::DoubleMap {
                ref hasher,
                ref key2_hasher,
                ..
            } => {
                if (k1.is_none() || k1.as_ref().unwrap().is_empty())
                    || (k2.is_none() || k2.as_ref().unwrap().is_empty())
                {
                    return Ok(StatusCode::BadRequest.into());
                }
                key.push_str(&hash(hasher, &k1.unwrap()));
                key.push_str(&hash(key2_hasher, &k2.unwrap()));
                key
            }
        }
    );

    rpc("state_getStorage", &[&key])
        .await
        .map(|res| response_from_type(mime, res))
}

fn response_from_type(mime: &Mime, res: String) -> Response {
    use base58::ToBase58;
    let bytes = || hex::decode(&res[2..]).unwrap();
    let mut res: Response = match mime.essence() {
        "text/plain" => res.into(),
        BASE58_MIME => bytes().to_base58().into(),
        _ => bytes().into(),
    };
    res.set_content_type(mime.clone());
    res
}

/// HTTP based JSONRpc request expecting an hex encoded result
async fn rpc(method: &str, params: &[&str]) -> Result<String> {
    surf::post(NODE_ENDPOINT)
        .content_type("application/json")
        .body(
            to_string(&jsonrpc::Request {
                id: 1.into(),
                jsonrpc: Some("2.0"),
                method,
                params: &params
                    .iter()
                    .map(|p| RawValue::from_string(p.to_string()).unwrap())
                    .collect::<Vec<_>>(),
            })
            .unwrap(),
        )
        .await
        .map_err(|_| Error::NodeConnection)?
        .body_json::<jsonrpc::Response>()
        .await
        .map_err(|_| Error::InvalidJSON)?
        .result()
        .map_err(Error::from)
}

pub enum Error {
    NodeConnection,
    InvalidJSON,
    Rpc(String),
    Decode(String),
    EmptyResponse,
    Unknown,
}

impl From<Error> for Response {
    fn from(e: Error) -> Self {
        match e {
            Error::NodeConnection => StatusCode::BadGateway.into(),
            Error::EmptyResponse => StatusCode::NotFound.into(),
            Error::Rpc(e) | Error::Decode(e) => {
                let mut res = Response::new(StatusCode::InternalServerError);
                res.set_body(e);
                res
            }
            _ => StatusCode::InternalServerError.into(),
        }
    }
}

impl From<hex::FromHexError> for Error {
    fn from(err: hex::FromHexError) -> Self {
        Error::Decode(err.to_string())
    }
}

impl From<jsonrpc::Error> for Error {
    fn from(err: jsonrpc::Error) -> Self {
        match err {
            jsonrpc::Error::Rpc(e) => Error::Rpc(e.message),
            jsonrpc::Error::Json(_) => Error::EmptyResponse,
            _ => Error::Unknown,
        }
    }
}

fn to_camel(term: &str) -> String {
    let underscore_count = term.chars().filter(|c| *c == '-').count();
    let mut result = String::with_capacity(term.len() - underscore_count);
    let mut at_new_word = true;

    for c in term.chars().skip_while(|&c| c == '-') {
        if c == '-' {
            at_new_word = true;
        } else if at_new_word {
            result.push(c.to_ascii_uppercase());
            at_new_word = false;
        } else {
            result.push(c);
        }
    }
    result
}
