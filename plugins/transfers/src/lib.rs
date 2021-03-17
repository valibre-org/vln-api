use serde::Deserialize;
use valor::*;

#[vlugin]
async fn transfers(req: Request) -> Response {
    match req.method() {
        Method::Get => StatusCode::NotFound.into(),
        Method::Post => decode_to_sign(req)
            .await
            .unwrap_or_else(|e| e.status().into()),
        _ => StatusCode::NotImplemented.into(),
    }
}

#[derive(Deserialize)]
struct Transfer {}

const SIGN_URL: &str = "/wallet/sign";

async fn decode_to_sign(mut req: Request) -> Result<Response, http::Error> {
    // TODO encode as SCALE
    let _t: Transfer = req.body_form().await?;
    let mut res: Response = StatusCode::SeeOther.into();
    res.append_header(http::headers::LOCATION, SIGN_URL);
    Ok(res)
}
