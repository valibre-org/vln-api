use std::env;

// use http::{content::Accept, mime, Mime};
use http::{content::Accept, mime};
use path_tree::PathTree;
use sube::{codec::Encode, http::Backend, Backend as _, Sube};
use valor::*;

static DEFAULT_NODE_URL: &str = "http://vln.valiu";
const SCALE_MIME: &str = "application/scale";
const BASE58_MIME: &str = "application/base58";

enum Cmd {
    Meta,
    Storage,
}

#[vlugin]
async fn blockchain(req: Request) -> Response {
    handler(req).await.unwrap_or_else(|err| err.status().into())
}

async fn handler(req: Request) -> Result<Response, http::Error> {
    let routes = {
        let mut p = PathTree::new();
        p.insert("/meta", Cmd::Meta);
        p.insert("/:module/:item", Cmd::Storage);
        p.insert("/:module/:item/*", Cmd::Storage);
        p
    };
    let url = req.url();
    let action = routes.find(url.path());
    if action.is_none() {
        return Ok(StatusCode::NotFound.into());
    }
    let (action, _params) = action.unwrap();

    let node_url = env::var("BLOCKCHAIN_NODE_URL").unwrap_or_else(|_| DEFAULT_NODE_URL.into());
    let node: Sube<_> = Backend::new(node_url.as_str()).into();
    let meta = node.try_init_meta().await?;

    // Use content negotiation to determine the response type
    // By default return data in SCALE encoded binary format
    let response_type: http::Mime = Accept::from_headers(&req)
        .expect("Valid Accept header")
        .unwrap_or_else(Accept::new)
        .negotiate(&[
            mime::PLAIN.essence().into(),
            SCALE_MIME.into(),
            BASE58_MIME.into(),
        ])
        .map(|c| c.value().as_str().into())
        .unwrap_or_else(|_| SCALE_MIME.into());

    Ok(match (req.method(), action) {
        (Method::Get, Cmd::Meta) => response_from_type(&response_type, &meta.encode()),
        (Method::Get, Cmd::Storage) => {
            let q = node.query_raw(url.path().trim_start_matches('/')).await?;
            response_from_type(&response_type, &q)
        }
        _ => StatusCode::MethodNotAllowed.into(),
    })
}

fn response_from_type(mime: &http::Mime, res: &[u8]) -> Response {
    use base58::ToBase58;
    let mut res: Response = match mime.essence() {
        "text/plain" => hex::encode(res).into(),
        BASE58_MIME => res.to_base58().into(),
        _ => res.into(),
    };
    res.set_content_type(mime.clone());
    res
}
