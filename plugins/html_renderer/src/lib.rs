mod template_renderer;

use http::convert::{json, Deserialize, Serialize};
use lazy_static::lazy_static;
use path_tree::PathTree;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use serde_json::Value as JsonValue;
use template_renderer::TemplateRenderer;
use valor::*;

#[derive(Deserialize, Serialize)]
struct RegisterTemplateInput {
    template: String,
}

#[derive(Deserialize, Serialize)]
struct RegisterTemplatePayload {
    template_id: String,
}

#[derive(Deserialize, Serialize)]
enum RenderOutput {
    Html,
    DataUrl,
}

#[derive(Deserialize, Serialize)]
struct RenderTemplateInput {
    template_id: String,
    data: Option<JsonValue>,
    output: Option<RenderOutput>,
}

enum Cmd {
    Templates,
    RenderTemplate,
    Unknown,
}

lazy_static! {
    static ref ROUTER: PathTree<Cmd> = {
        let mut p = PathTree::new();
        p.insert("/templates", Cmd::Templates);
        p.insert("/render", Cmd::RenderTemplate);
        p.insert("*", Cmd::Unknown);
        p
    };
}

#[vlugin]
#[derive(Default)]
struct Handler<'a> {
    html_renderer: TemplateRenderer<'a>,
}

#[async_trait(?Send)]
impl RequestHandler for Handler<'_> {
    async fn handle_request(&self, mut req: Request) -> Response {
        let (action, _params) = {
            ROUTER
                .find(req.url().path())
                .unwrap_or_else(|| (&Cmd::Unknown, vec![]))
        };

        match (action, req.method()) {
            (Cmd::Templates, Method::Get) => json!(self.html_renderer.get_templates()).into(),
            (Cmd::Templates, Method::Post) => {
                let template = match req.body_json().await {
                    Ok(RegisterTemplateInput { template }) => template,
                    _ => return StatusCode::BadRequest.into(),
                };

                let template_id = match self.html_renderer.register_template(&template) {
                    Ok(template_id) => template_id,
                    Err(_) => return StatusCode::BadRequest.into(),
                };

                let response = {
                    let mut response = Response::new(StatusCode::Created);
                    let body = match Body::from_json(&RegisterTemplatePayload { template_id }) {
                        Ok(body) => body,
                        _ => return StatusCode::InternalServerError.into(),
                    };
                    response.set_body(body);
                    response
                };
                response
            }
            (Cmd::RenderTemplate, Method::Post) => {
                let (template_id, data, output) = match req.body_json().await {
                    Ok(RenderTemplateInput {
                        template_id,
                        data,
                        output,
                    }) => (template_id, data, output),
                    _ => return StatusCode::BadRequest.into(),
                };

                let rendered_template =
                    match self.html_renderer.render_template(&template_id, &data) {
                        Some(rendered_template) => rendered_template,
                        _ => return StatusCode::BadRequest.into(),
                    };

                match output {
                    Some(RenderOutput::DataUrl) => format!(
                        "data:text/html,{}",
                        utf8_percent_encode(&rendered_template, NON_ALPHANUMERIC)
                    ),
                    _ => rendered_template,
                }
                .into()
            }
            (Cmd::Templates, _) | (Cmd::RenderTemplate, _) => StatusCode::MethodNotAllowed.into(),
            (Cmd::Unknown, _) => StatusCode::NotFound.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[async_std::test]
    async fn it_registers_template() {
        let handler = super::Handler::default();
        let template = "<p>Hello, world.</p>";
        let template_id = register_template(&handler, template.into()).await;

        assert!(template_id.len() > 0);
    }

    #[async_std::test]
    async fn it_stores_registered_template() {
        let handler = super::Handler::default();
        let template = "<p>Hello, world.</p>";
        let template_id = register_template(&handler, template.into()).await;
        let request = Request::new(Method::Get, "http://localhost/templates");
        let mut response = handler.handle_request(request).await;
        let output: Vec<String> = response.body_json().await.unwrap();

        assert_eq!(output, vec![template_id.clone()]);
    }

    #[async_std::test]
    async fn it_renders_registered_template_with_data() {
        let handler = super::Handler::default();
        let template = "<p>Hello, {{firstname}} {{lastname}}.</p>";
        let template_id = register_template(&handler, template.into()).await;

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
        let mut response = handler.handle_request(request).await;
        let output = response.body_string().await.unwrap();

        assert_eq!(output, "<p>Hello, John Doe.</p>");
    }

    async fn register_template<'a>(handler: &super::Handler<'a>, template: &str) -> String {
        let request = {
            let input = RegisterTemplateInput {
                template: template.into(),
            };
            let mut request = Request::new(Method::Post, "http://localhost/templates");
            let request_body = Body::from_json(&input).unwrap();
            request.set_body(request_body);
            request
        };

        let mut response = handler.handle_request(request).await;
        let RegisterTemplatePayload { template_id } = response.body_json().await.unwrap();
        template_id
    }
}
