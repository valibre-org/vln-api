# Capture Url Plugin

Generate screenshots from any website.

## How to use it

```sh
curl --header "Content-Type: application/json" \
  --request POST \
  --data '{ "url": "https://google.com", "dimensions": { "type" : "Phone" } }' \
  http://runtime.path/capture_url/capture-url
```

The response will be the image. If `--header "Accept: text/plain"` is specified, then the response will be the base64 encoded image.

Possible values for `dimensions` are:

- `{ "type": "Phone" }`: 450 (width) x 800 (height)
- `{ "type": "Tablet" }`: 1024 x 768
- `{ "type": "Desktop" }`: 1920 x 1080
- `{ "type": "CustomDimensions", width: number, height: number }`: `width` x `height`

## Tests

The tests in this plugin require a running instance of `geckodriver` at address specified in `CAPTURE_URL_WEB_DRIVER_HOST` (defaults to `http://localhost:4444`)

### To run the tests

```sh
cargo test -- --test-threads=1
```

We need to limit the tests to a single thread because the selenium server only supports 1 browser instance at a time.
