# Web

A dead simple web server for serving static files.

## Configurations

The server can be configured using environment variables:

- `MY_SITE_WEB_ADDR`
  The IP address the server listens on.
  Default: `0.0.0.0`

- `MY_SITE_WEB_PORT`
  The port the server listens on.
  Default: `5000`

- `MY_SITE_WEB_SERVED_DIR_PATH`
  The directory path to serve files from.
  Default: `/data`

- `MY_SITE_WEB_NOT_FOUND_PAGE_FILE_PATH`
  The file to serve when a requested file is not found.
  Default: `not_found.html`
