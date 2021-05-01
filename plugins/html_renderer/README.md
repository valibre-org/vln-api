# Template Renderer Plugin

Register and render handlebar templates.

## Usage

1. Register template.

```sh
$ curl http://runtime.path/html_renderer/templates \
  -H "Content-Type: application/json" \
  -d '{ "template": "<p>Hello, {{#if name}}{{name}}{{else}}world{{/if}}</p>" }'

{"template_id": "11043029071595622528"}
```

2. Render template

```sh
$ curl http://runtime.path/html_renderer/render \
  -H "Content-Type: application/json" \
  -d \
  '{
     "template_id": "11043029071595622528",
     "data": { "name": "John Doe" },
     "output": { "type": "DataUrl" }
   }'

<p>Hello, John Doe</p>


# Use `"output": { "type": "DataUrl" }` to get the rendered template as Data Url. It defaults to `{ "type": "Html" }`
$ curl http://runtime.path/html_renderer/render \
  -H "Content-Type: application/json" \
  -d \
  '{
     "template_id": "11043029071595622528",
     "data": { "name": "John Doe" },
     "output": { "type": "DataUrl" }
   }'

data:text/html,%3Cp%3EHello%2C%20John%20Doe%3C%2Fp%3E


# Specify a redirect url when `output.type == 'DataUrl'` and the query param name (defaults to 'url')
# which the rendered data url will be assigned to. Optional.
$ curl -v http://runtime.path/html_renderer/render \
  -H "Content-Type: application/json" \
  -d \
  '{
     "template_id": "11043029071595622528",
     "data": { "name": "John Doe" },
     "output": {
       "type": "DataUrl",
       "redirect_url": "https://render-url.com",
       "query_param_name": "target"
     }
   }' \

# ...
< HTTP/1.1 303 See Other
# ...
< location: https://render-url.com/?target=data%3Atext%2Fhtml%2C%253Cp%253EHello%252C%2520John%2520Doe%253C%252Fp%253E
# ...
```
