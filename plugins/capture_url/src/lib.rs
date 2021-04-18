use http::{content::Accept, mime, Method, Request, Response, StatusCode};
use std::collections::HashMap;
use std::num::ParseIntError;
use std::str::FromStr;
use thirtyfour::{prelude::*, OptionRect};
use url::Url;
use valor::*;

const LOCAL_WEB_DRIVER_HOST: &str = "http://localhost:4444/wd/hub";

#[vlugin]
pub async fn on_create(cx: &mut Context) {
    cx.set(
        cx.config::<String>()
            .map(|host| host.parse::<Url>().ok())
            .flatten()
            .unwrap_or_else(|| LOCAL_WEB_DRIVER_HOST.parse().unwrap()),
    );
}

pub async fn on_request(cx: &Context, req: Request) -> Response {
    let host = cx.get::<Url>();
    let url = req.url();
    let method = req.method();

    match (url.path(), method) {
        ("/", Method::Get) => capture_handler(&host, req).await,
        ("/", _) => StatusCode::MethodNotAllowed.into(),
        _ => StatusCode::NotFound.into(),
    }
}

struct ImageDimensions {
    width: u16,
    height: u16,
}

impl FromStr for ImageDimensions {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let dimensions: Vec<&str> = s
            .trim_matches(|p| p == '(' || p == ')')
            .split(',')
            .collect();

        Ok(ImageDimensions {
            width: dimensions[0].parse::<u16>()?,
            height: dimensions[1].parse::<u16>()?,
        })
    }
}

async fn capture_url(
    host: &Url,
    url: &Url,
    dimensions: &Option<ImageDimensions>,
) -> WebDriverResult<Vec<u8>> {
    let caps = {
        let mut caps = DesiredCapabilities::firefox();
        caps.set_headless()?;
        caps
    };

    let driver = WebDriver::new(host.as_str(), &caps).await?;
    let ImageDimensions { width, height } =
        dimensions.as_ref().unwrap_or_else(|| &ImageDimensions {
            width: 450,
            height: 800,
        });

    // The navigation bar has a height of 74 px. It is excluded when taking a screenshot (something desirable) making
    // the screenshot's height less than the value specified in `dimensions`. By adding its height when setting
    // the window's height, the screenshot's height will match the dimension given.
    // See https://github.com/mozilla/geckodriver/issues/1744
    const NAVIGATION_BAR_HEIGHT: i32 = 74;
    driver
        .set_window_rect(OptionRect::new().with_size(
            i32::from(*width),
            i32::from(*height) + NAVIGATION_BAR_HEIGHT,
        ))
        .await?;

    driver.get(url.as_str()).await?;
    driver.screenshot_as_png().await
}

async fn capture_handler(host: &Url, req: Request) -> Response {
    let hash_query: HashMap<_, _> = req.url().query_pairs().into_owned().collect();
    let url = hash_query.get("url");
    let dimensions = hash_query
        .get("dimensions")
        .and_then(|dimensions| dimensions.parse::<ImageDimensions>().ok());

    let response_mime = Accept::from_headers(&req)
        .unwrap_or_default()
        .unwrap_or_else(Accept::new)
        .negotiate(&[mime::PNG.essence().into(), mime::PLAIN.essence().into()])
        .map(|c| c.value().as_str().into())
        .unwrap_or_else(|_| mime::PNG);

    let url = match url {
        Some(url) => url,
        _ => return StatusCode::BadRequest.into(),
    };

    let url = match url.parse::<Url>() {
        Ok(url) => url,
        _ => return StatusCode::BadRequest.into(),
    };

    let image = match capture_url(host, &url, &dimensions).await {
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

#[cfg(test)]
mod tests {
    use super::*;
    use image::GenericImageView;

    #[async_std::test]
    async fn it_returns_a_correct_png_image_with_given_dimensions() {
        let dimensions = ImageDimensions {
            width: 450,
            height: 800,
        };

        let request = {
            let url_to_capture = "http://webserver";
            let url = Url::parse_with_params(
                "http://localhost.test",
                &[
                    ("url", url_to_capture),
                    (
                        "dimensions",
                        &format!("{},{}", dimensions.width, dimensions.height),
                    ),
                ],
            )
            .unwrap();
            let request = Request::new(Method::Get, url);
            request
        };

        let mut response = on_request(&test_context().await, request).await;
        let buffer = response.body_bytes().await.unwrap();
        let image = image::load_from_memory_with_format(&buffer, image::ImageFormat::Png).unwrap();
        let actual_dimensions = image.dimensions();
        let expected_dimensions = {
            let ImageDimensions { width, height } = dimensions;
            (u32::from(width), u32::from(height))
        };

        assert_eq!(actual_dimensions, expected_dimensions);
    }

    #[async_std::test]
    async fn it_returns_a_correct_base64_encoded_png_image_with_given_dimensions() {
        let dimensions = ImageDimensions {
            width: 1024,
            height: 768,
        };

        let request = {
            let url_to_capture = "http://webserver";
            let url = Url::parse_with_params(
                "http://localhost.test",
                &[
                    ("url", url_to_capture),
                    (
                        "dimensions",
                        &format!("{},{}", dimensions.width, dimensions.height),
                    ),
                ],
            )
            .unwrap();
            let mut request = Request::new(Method::Get, url);
            request.insert_header("Accept", "text/plain");
            request
        };

        let mut response = on_request(&test_context().await, request).await;
        let base64_image = response.body_string().await.unwrap();
        let splitted_data_uri: Vec<&str> = base64_image.split(',').collect();

        // check if data uri schema is the one expected
        let data_schema = *splitted_data_uri.get(0).unwrap();
        assert_eq!(data_schema, "data:image/png;base64");

        // check if base64 data is a valid png
        let image_data = *splitted_data_uri.get(1).unwrap();
        let buffer = &base64::decode(image_data).unwrap();
        let image = image::load_from_memory_with_format(buffer, image::ImageFormat::Png).unwrap();
        let actual_dimensions = image.dimensions();
        let expected_dimensions = {
            let ImageDimensions { width, height } = dimensions;
            (u32::from(width), u32::from(height))
        };

        assert_eq!(actual_dimensions, expected_dimensions);
    }

    async fn test_context() -> Context {
        let mut cx = Context::default();
        on_create(&mut cx).await;
        cx
    }
}
