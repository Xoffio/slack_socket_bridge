Create a `.env` file with the next env variables:

- `WEBHOOK_URL_PROD`: Production Webhook URL to send data to.
- `WEBHOOK_URL_DEV`: Development Webhook URL to send data to.
- `WEBHOOK_URL_CMD_PROD`: Production Webhook URL to send data to (command events).
- `WEBHOOK_URL_CMD_DEV`: Development Webhook URL to send data to (command events).
- `SLACK_SOCKET_TOKEN`: Slack socket token

To build the image and run it locally run the next commands:

```bash
podman build -t slack_socket_bridge .
podman run --env-file=.env localhost/slack_socket_bridge:latest
```
