# Capture Url Plugin

Generate screenshots from any website.

## How to use it

```sh
curl --header "Content-Type: application/json" \
  "http://runtime.path/capture_url?url=https://google.com&dimensions=1920,1080" \
  --output image.png
```

The response will be the image. If `--header "Accept: text/plain"` is specified, then the response will be the base64 encoded image.

```sh
curl --header "Content-Type: application/json" \
  "http://runtime.path/capture_url?url=https://google.com&dimensions=1024,768" \
  --header "Accept: text/plain" \
  --output image-as-base64.txt
```

Possible query parameters are:

- url: [Percentage-encoded](https://developer.mozilla.org/en-US/docs/Glossary/percent-encoding) URL to capture the image from. Most HTTP client libs encode query parameters automatically. [Data URLs](https://developer.mozilla.org/en-US/docs/Web/HTTP/Basics_of_HTTP/Data_URIs) are valid values (once encoded).
- dimensions: Comma separated width and height values of the image, eg. `dimensions=width,height` or `dimensions=(width,height)`. Optional. Defaults to `450,800`.

## Tests

The tests in this plugin require a running instance of geckodriver or selenium at address specified in `CAPTURE_URL_WEB_DRIVER_HOST` (defaults to `http://localhost:4444/wd/hub`). A `docker-compose.yml` is included and it will setup Selenium and a sample web server.

### To run the tests

```sh
docker-compose up
cargo test
```
