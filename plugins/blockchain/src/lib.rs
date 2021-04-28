use base58::ToBase58;
use http::Status;
use http::{content::Accept, Method, Request, Response, StatusCode};
use path_tree::PathTree;
use sube::{codec::Encode, http::Backend, Backend as _, Sube};
use valor::*;

type Router = PathTree<Cmd>;

enum Cmd {
    Meta,
    Storage,
}

const DEFAULT_NODE_URL: &str = "http://localhost:9933";
const VALID_MIMES: &[&str] = &[
    "application/base58",
    "application/json",
    "application/scale",
    "text/plain",
];
const BASE58_MIME: &str = VALID_MIMES[0];
const JSON_MIME: &str = VALID_MIMES[1];
const SCALE_MIME: &str = VALID_MIMES[2];

#[vlugin]
pub async fn on_create(cx: &mut Context) {
    cx.set({
        let mut r = Router::new();
        r.insert("/meta", Cmd::Meta);
        r.insert("/:module/:item", Cmd::Storage);
        r.insert("/:module/:item/*", Cmd::Storage);
        r
    });
    cx.set(
        VALID_MIMES
            .iter()
            .map(|m| http::Mime::from(*m))
            .collect::<Vec<_>>(),
    );
}

pub async fn on_request(cx: &Context, req: Request) -> http::Result<Response> {
    let routes = cx.get::<Router>();
    let url = req.url();
    let action = routes.find(url.path());
    if action.is_none() {
        return Ok(http::StatusCode::NotFound.into());
    }
    let (action, _params) = action.unwrap();

    let node_url = cx
        .raw_config()
        .map(VluginConfig::as_str)
        .flatten()
        .unwrap_or(DEFAULT_NODE_URL);
    let node: Sube<_> = Backend::new(node_url).into();
    let meta = node.try_init_meta().await.status(StatusCode::BadGateway)?;

    // Use content negotiation to determine the response type
    // By default return data in SCALE encoded binary format
    let mime_res: http::Mime = Accept::from_headers(&req)
        .expect("Valid Accept header")
        .unwrap_or_else(Accept::new)
        .negotiate(cx.get::<Vec<_>>())
        .map(|c| c.value().as_str().into())
        .unwrap_or_else(|_| SCALE_MIME.into());

    Ok(match (req.method(), action) {
        (Method::Get, Cmd::Meta) => {
            let mut res: Response = match mime_res.essence() {
                SCALE_MIME => meta.encode().into(),
                BASE58_MIME => meta.encode().to_base58().into(),
                #[cfg(feature = "serde_json")]
                JSON_MIME => serde_json::to_string(meta)?.into(),
                _ => hex::encode(meta.encode()).into(),
            };
            res.set_content_type(mime_res);
            res
        }
        (Method::Get, Cmd::Storage) => {
            let q = node.query_raw(url.path().trim_start_matches('/')).await?;
            let mut res: Response = match mime_res.essence() {
                SCALE_MIME => q.into(),
                BASE58_MIME => q.to_base58().into(),
                JSON_MIME => todo!(),
                _ => hex::encode(q).into(),
            };
            res.set_content_type(mime_res);
            res
        }
        _ => StatusCode::MethodNotAllowed.into(),
    })
}
