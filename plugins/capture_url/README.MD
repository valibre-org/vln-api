# Capture Url Plugin

Generate screenshots from any website.

## How to use it

```sh
curl --header "Content-Type: application/json" \
  --data '{ "url": "https://google.com", "dimensions": { "type" : "Phone" } }' \
  --output image.png \
  http://runtime.path/_capture_url/capture-url
```

The response will be the image. If `--header "Accept: text/plain"` is specified, then the response will be the base64 encoded image.

```sh
curl --header "Content-Type: application/json" \
  --data '{ "url": "https://google.com", "dimensions": { "type" : "Phone" } }' \
  --header "Accept: text/plain" \
  --output image-as-base64.txt \
  http://runtime.path/_capture_url/capture-url
```

Possible values for `dimensions` are:

- `{ "type": "Phone" }`: 450 (width) x 800 (height)
- `{ "type": "Tablet" }`: 1024 x 768
- `{ "type": "Desktop" }`: 1920 x 1080
- `{ "type": "CustomDimensions", width: number, height: number }`: `width` x `height`

## Tests

The tests in this plugin require a running instance of geckodriver or selenium at address specified in `CAPTURE_URL_WEB_DRIVER_HOST` (defaults to `http://localhost:4444/wd/hub`). A `docker-compose.yml` is included and it will setup Selenium and a sample web server.

### To run the tests

```sh
docker-compose up
cargo test
```
