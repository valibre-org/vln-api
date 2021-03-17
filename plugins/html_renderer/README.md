# Template Renderer Plugin

Register and render handlebar templates.

## Usage

1. Register template.

  ```sh
  $ curl --header "Content-Type: application/json" \
    --data '{ "template": "<p>Hello, {{#if name}}{{name}}{{else}}world{{/if}}</p>" }' \
    http://runtime.path/html_renderer/templates

  {"template_id": "11043029071595622528"}
  ```

2. Render template

  ```sh
  $ curl --header "Content-Type: application/json" \
    --data '{ "template_id": "11043029071595622528", "data": { "name": "John Doe" } }' \
    http://runtime.path/html_renderer/render

  <p>Hello, John Doe</p>

  # Use `output: "DataUrl"` to get the rendered template as Data Url. It defaults to `Html`
  $ curl --header "Content-Type: application/json" \
    --data '{ "template_id": "11043029071595622528", "data": { "name": "John Doe", output: "DataUrl" } }' \
    http://runtime.path/html_renderer/render

  data:text/html,%3Cp%3EHello%2C%20John%20Doe%3C%2Fp%3E
  ```
