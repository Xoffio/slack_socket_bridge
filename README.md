Create a `.env` file with the next env variables:

- `N8N_WEBHOOK_URL`: Webhook URL created by n8n.
- `SLACK_SOCKET_TOKEN`: Slack socket token

To build the image and run it locally run the next commands:

```bash
podman build -t slack_socket_bridge .
podman run --env-file=.env localhost/slack_socket_bridge:latest
```
