use lazy_static::lazy_static;
use path_tree::PathTree;
use std::env;
use thirtyfour::{prelude::*, OptionRect};
use valor::{
    http::{
        content::Accept,
        convert::{Deserialize, Serialize},
        mime, Request, Response,
    },
    *,
};

#[derive(Clone, Deserialize, Serialize)]
#[serde(tag = "type")]
enum WindowDimensions {
    Phone,
    Tablet,
    Desktop,
    CustomDimensions { width: u16, height: u16 },
}

impl WindowDimensions {
    fn get_size_in_pixel(&self) -> (u16, u16) {
        match self {
            Self::Desktop => (1920, 1080),
            Self::Tablet => (1024, 768),
            Self::Phone => (450, 800),
            Self::CustomDimensions { width, height } => (*width, *height),
        }
    }
}

#[derive(Deserialize, Serialize)]
struct Input {
    url: String,
    dimensions: Option<WindowDimensions>,
}

enum Cmd {
    Capture,
    Unknown,
}

lazy_static! {
    static ref ROUTER: PathTree<Cmd> = {
        let mut p = PathTree::new();
        p.insert("/capture-url", Cmd::Capture);
        p.insert("*", Cmd::Unknown);
        p
    };
    static ref WEB_DRIVER_HOST: String = {
        let host = match env::var("CAPTURE_URL_WEB_DRIVER_HOST") {
            Ok(host) => host,
            Err(_) => "http://localhost:4444".to_owned(),
        };

        let _ = Url::parse(&host).expect("Env var `WEB_DRIVER_HOST` expected to be a valid URL");
        host
    };
}

async fn capture_url(url: &Url, dimensions: &Option<WindowDimensions>) -> WebDriverResult<Vec<u8>> {
    let caps = {
        let mut caps = DesiredCapabilities::firefox();
        caps.set_headless()?;
        caps
    };

    let driver = WebDriver::new(&WEB_DRIVER_HOST, &caps).await?;
    let (width, height) = dimensions
        .as_ref()
        .unwrap_or_else(|| &WindowDimensions::Desktop)
        .get_size_in_pixel();

    // The navigation bar has a height of 74 px. It is excluded when taking a screenshot (something desirable) making
    // the screenshot's height less than the value specified in `dimensions`. By adding its height when setting
    // the window's height, the screenshot's height will match the dimension given.
    // See https://github.com/mozilla/geckodriver/issues/1744
    const NAVIGATION_BAR_HEIGHT: i32 = 74;
    driver
        .set_window_rect(
            OptionRect::new()
                .with_size(i32::from(width), i32::from(height) + NAVIGATION_BAR_HEIGHT),
        )
        .await?;

    driver.get(url.as_str()).await?;
    driver.screenshot_as_png().await
}

async fn capture_handler(mut req: Request) -> Response {
    let response_mime = Accept::from_headers(&req)
        .unwrap_or_default()
        .unwrap_or_else(Accept::new)
        .negotiate(&[mime::PNG.essence().into(), mime::PLAIN.essence().into()])
        .map(|c| c.value().as_str().into())
        .unwrap_or_else(|_| mime::PNG);

    let (url, dimensions) = match req.body_json().await {
        Ok(Input { url, dimensions }) => (url, dimensions),
        _ => return StatusCode::BadRequest.into(),
    };

    let url = match url.parse::<Url>() {
        Ok(url) => url,
        _ => return StatusCode::BadRequest.into(),
    };

    let image = match capture_url(&url, &dimensions).await {
        Ok(image) => image,
        _ => return StatusCode::InternalServerError.into(),
    };

    let mut res: Response = match response_mime.essence() {
        "text/plain" => format!(
            "data:{};base64,{}",
            mime::PNG.essence(),
            base64::encode(image)
        )
        .into(),
        "image/png" | _ => image.into(),
    };

    res.set_content_type(response_mime.clone());
    res
}

#[vlugin]
async fn handler(req: Request) -> Response {
    let url = req.url();

    let (action, _params) = ROUTER
        .find(url.path())
        .unwrap_or_else(|| (&Cmd::Unknown, vec![]));

    match (action, req.method()) {
        (Cmd::Capture, Method::Post) => capture_handler(req).await,
        (Cmd::Capture, _) => StatusCode::MethodNotAllowed.into(),
        (Cmd::Unknown, _) => StatusCode::NotFound.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::GenericImageView;

    #[async_std::test]
    async fn it_returns_a_correct_png_image_with_given_dimensions() {
        let server = setup_local_server();

        let dimensions = WindowDimensions::Phone;
        let input = Input {
            url: server.url_str("/"),
            dimensions: Some(dimensions.clone()),
        };

        let request = {
            let mut request = Request::new(Method::Post, "http://localhost/capture-url");
            let request_body = Body::from_json(&input).unwrap();
            request.set_body(request_body);
            request
        };

        let mut response = handler(request).await;
        let buffer = response.body_bytes().await.unwrap();
        let image = image::load_from_memory_with_format(&buffer, image::ImageFormat::Png).unwrap();
        let (actual_width, actual_height) = image.dimensions();
        let (expected_width, expected_height) = {
            let (width, height) = dimensions.get_size_in_pixel();
            (u32::from(width), u32::from(height))
        };

        assert!(expected_width == actual_width);
        assert!(expected_height == actual_height);
    }

    #[async_std::test]
    async fn it_returns_a_correct_base64_encoded_png_image_with_given_dimensions() {
        let server = setup_local_server();

        let dimensions = WindowDimensions::Tablet;
        let input = Input {
            url: server.url_str("/"),
            dimensions: Some(dimensions.clone()),
        };

        let request = {
            let mut request = Request::new(Method::Post, "http://localhost/capture-url");
            request.insert_header("Accept", "text/plain");
            let request_body = Body::from_json(&input).unwrap();
            request.set_body(request_body);
            request
        };

        let mut response = handler(request).await;
        let base64_image = response.body_string().await.unwrap();
        let splitted_data_uri: Vec<&str> = base64_image.split(',').collect();

        // check if data uri schema is the one expected
        let data_schema = *splitted_data_uri.get(0).unwrap();
        assert!(data_schema == "data:image/png;base64");

        // check if base64 data is a valid png
        let image_data = *splitted_data_uri.get(1).unwrap();
        let buffer = &base64::decode(image_data).unwrap();
        let image = image::load_from_memory_with_format(buffer, image::ImageFormat::Png).unwrap();
        let (actual_width, actual_height) = image.dimensions();
        let (expected_width, expected_height) = {
            let (width, height) = dimensions.get_size_in_pixel();
            (u32::from(width), u32::from(height))
        };

        assert!(expected_width == actual_width);
        assert!(expected_height == actual_height);
    }

    fn setup_local_server() -> httptest::Server {
        use httptest::{matchers::*, responders::*, Expectation, Server};

        let server = Server::run();
        server.expect(
            Expectation::matching(any()).times(..).respond_with(
                status_code(200)
                    .append_header("Content-Type", "text/html")
                    .body("<h1>Hello, world!</h1><h2>Capture this.</h2>"),
            ),
        );

        server
    }
}
