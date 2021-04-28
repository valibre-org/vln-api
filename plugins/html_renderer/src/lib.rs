mod template_renderer;
mod utils;

use http::{
    convert::{json, Deserialize, Serialize},
    Body, Method, Request, Response, Status, StatusCode,
};
use serde_json::Value as JsonValue;
use template_renderer::TemplateRenderer;
use url::Url;
use utils::format_html_data_url;
use valor::*;

#[derive(Deserialize, Serialize)]
struct RegisterTemplateInput {
    template: String,
}

#[derive(Deserialize, Serialize)]
struct RegisterTemplatePayload {
    template_id: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
enum RenderOutput {
    Html,
    DataUrl {
        redirect_url: Option<String>,
        query_param_name: Option<String>,
    },
}

#[derive(Deserialize, Serialize)]
struct RenderTemplateInput {
    template_id: String,
    data: Option<JsonValue>,
    output: Option<RenderOutput>,
}

#[vlugin]
pub async fn on_create(cx: &mut Context) {
    cx.set(TemplateRenderer::default());
}

pub async fn on_request(cx: &Context, mut req: Request) -> http::Result<Response> {
    let html_renderer = cx.get::<TemplateRenderer>();

    let response = match (req.url().path(), req.method()) {
        ("/templates", Method::Get) => json!(html_renderer.get_templates()).into(),
        ("/templates", Method::Post) => {
            let RegisterTemplateInput { template } = req.body_json().await?;
            let template_id = html_renderer
                .register_template(&template)
                .map_err(|_| http::Error::from_str(StatusCode::BadRequest, ""))?;
            let response = {
                let mut response = Response::new(StatusCode::Created);
                let body = Body::from_json(&RegisterTemplatePayload { template_id })?;
                response.set_body(body);
                response
            };
            response
        }
        ("/render", Method::Post) => {
            let RenderTemplateInput {
                template_id,
                data,
                output,
            } = req.body_json().await?;

            let rendered_template = html_renderer
                .render_template(&template_id, &data)
                .status(StatusCode::BadRequest)?;

            match output {
                Some(RenderOutput::DataUrl {
                    redirect_url: Some(redirect_url),
                    query_param_name,
                }) => {
                    let data_url = format_html_data_url(&rendered_template);
                    let query_param_name = query_param_name.unwrap_or("url".into());
                    let url =
                        Url::parse_with_params(&redirect_url, &[(query_param_name, &data_url)])?;
                    let mut response = Response::new(StatusCode::SeeOther);
                    let _ = response.insert_header("Location", url.to_string());
                    response
                }
                Some(RenderOutput::DataUrl { .. }) => {
                    format_html_data_url(&rendered_template).into()
                }
                _ => rendered_template.into(),
            }
        }
        ("/templates", _) | ("/render", _) => StatusCode::MethodNotAllowed.into(),
        _ => StatusCode::NotImplemented.into(),
    };

    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[async_std::test]
    async fn it_registers_template() {
        let template = "<p>Hello, world.</p>";
        let template_id = register_template(&test_context().await, template.into()).await;

        assert!(template_id.len() > 0);
    }

    #[async_std::test]
    async fn it_stores_registered_template() {
        let template = "<p>Hello, world.</p>";
        let cx = test_context().await;
        let template_id = register_template(&cx, template.into()).await;
        let request = Request::new(Method::Get, "http://localhost/templates");
        let mut response = on_request(&cx, request).await.unwrap();
        let output: Vec<String> = response.body_json().await.unwrap();

        assert_eq!(output, vec![template_id.clone()]);
    }

    #[async_std::test]
    async fn it_renders_registered_template_with_data() {
        let template = "<p>Hello, {{firstname}} {{lastname}}.</p>";
        let cx = test_context().await;
        let template_id = register_template(&cx, template.into()).await;

        dbg!(&template_id);

        let request = {
            let input = RenderTemplateInput {
                template_id: template_id.clone(),
                data: Some(json!({ "firstname": "John", "lastname": "Doe" })),
                output: None,
            };
            let mut request = Request::new(Method::Post, "http://localhost/render");
            let request_body = Body::from_json(&input).unwrap();
            request.set_body(request_body);
            request
        };
        let mut response = on_request(&cx, request).await.unwrap();
        let output = response.body_string().await.unwrap();

        assert_eq!(output, "<p>Hello, John Doe.</p>");
    }

    #[async_std::test]
    async fn it_redirects_to_url_with_image_data() {
        let template = "<p>Hello, {{firstname}} {{lastname}}.</p>";
        let cx = test_context().await;
        let template_id = register_template(&cx, template.into()).await;

        let request = {
            let input = RenderTemplateInput {
                template_id: template_id.clone(),
                data: Some(json!({ "firstname": "John", "lastname": "Doe" })),
                output: Some(RenderOutput::DataUrl {
                    redirect_url: Some("https://test.com".into()),
                    query_param_name: Some("data_url".into()),
                }),
            };
            let mut request = Request::new(Method::Post, "http://localhost/render");
            let request_body = Body::from_json(&input).unwrap();
            request.set_body(request_body);
            request
        };

        let response = on_request(&cx, request).await.unwrap();
        assert_eq!(response.status(), StatusCode::SeeOther);

        let location = response
            .header("Location")
            .unwrap()
            .get(0)
            .unwrap()
            .as_str();
        let expected_location = "https://test.com/?data_url=data%3Atext%2Fhtml%2C%253Cp%253EHello%252C%2520John%2520Doe%252E%253C%252Fp%253E";
        assert_eq!(location, expected_location);
    }

    async fn register_template<'a>(cx: &Context, template: &str) -> String {
        let request = {
            let input = RegisterTemplateInput {
                template: template.into(),
            };
            let mut request = Request::new(Method::Post, "http://localhost/templates");
            let request_body = Body::from_json(&input).unwrap();
            request.set_body(request_body);
            request
        };

        let mut response = on_request(cx, request).await.unwrap();
        let RegisterTemplatePayload { template_id } = response.body_json().await.unwrap();
        template_id
    }

    async fn test_context() -> Context {
        let mut cx = Context::default();
        on_create(&mut cx).await;
        cx
    }
}
