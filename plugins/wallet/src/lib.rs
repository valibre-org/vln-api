use path_tree::PathTree;
use valor::*;

enum Cmd {
    Open,
}

type Result<T> = std::result::Result<T, Error>;

#[vlugin]
async fn wallet(req: Request) -> Response {
    let routes = {
        let mut p = PathTree::new();
        p.insert("/open", Cmd::Open);
        p
    };
    let url = req.url();
    let action = routes.find(url.path());
    if action.is_none() {
        return StatusCode::NotFound.into();
    }
    let (action, _params) = action.unwrap();

    match (req.method(), action) {
        (Method::Get, Cmd::Open) => open_wallet().await,
        _ => Ok(StatusCode::MethodNotAllowed.into()),
    }
    .unwrap_or_else(Into::into)
}

async fn open_wallet() -> Result<Response> {
    todo!()
}

pub enum Error {
    Unknown,
}

impl From<Error> for Response {
    fn from(e: Error) -> Self {
        match e {
            _ => StatusCode::InternalServerError.into(),
        }
    }
}
