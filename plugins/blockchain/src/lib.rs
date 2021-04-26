use http::{content::Accept, mime, Method, Request, Response, StatusCode};
use path_tree::PathTree;
use sube::{codec::Encode, http::Backend, Backend as _, Sube};
use valor::*;

type Router = PathTree<Cmd>;

enum Cmd {
    Meta,
    Storage,
}

static DEFAULT_NODE_URL: &str = "http://vln.valiu";
const SCALE_MIME: &str = "application/scale";
const BASE58_MIME: &str = "application/base58";

#[vlugin]
pub async fn on_create(cx: &mut Context) {
    cx.set({
        let mut r = Router::new();
        r.insert("/meta", Cmd::Meta);
        r.insert("/:module/:item", Cmd::Storage);
        r.insert("/:module/:item/*", Cmd::Storage);
        r
    });
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
