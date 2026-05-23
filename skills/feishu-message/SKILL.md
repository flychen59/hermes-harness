---
name: feishu-message
description: |
  Send private messages to Feishu users via the Feishu IM API. Use when you need to
  proactively send a direct message to a specific user by open_id, outside of the
  normal reply flow. Supports text, rich text (post), and interactive card messages.
---

# Feishu Private Message

Send messages directly to Feishu users via the IM v1 API.

## When to Use

- Proactively notify a user (not in reply to an incoming message)
- Send alerts, reminders, or updates to a specific person
- Broadcast a message to multiple users individually

## Prerequisites

- Feishu bot app must have **bot capability** enabled
- Bot must have scope: `im:message` or `im:message:send_as_bot`
- Recipient must be within the bot's **availability scope**

## Send a Text Message

```bash
bash "$(dirname "$0")/scripts/send_message.sh" "ou_xxx" "Hello, this is a private message!"
```

## Send a Rich Text (Post) Message

```bash
bash "$(dirname "$0")/scripts/send_message.sh" "ou_xxx" \
  '{"title":"Alert","content":[[{"tag":"text","text":"Something happened"},{"tag":"a","text":"Details","href":"https://example.com"}]]}' \
  post
```

## Send an Interactive Card Message

```bash
bash "$(dirname "$0")/scripts/send_message.sh" "ou_xxx" \
  '{"config":{"wide_screen_mode":true},"header":{"title":{"tag":"plain_text","content":"Notification"},"template":"blue"},"elements":[{"tag":"div","text":{"tag":"lark_md","content":"Your task is complete."}}]}' \
  interactive
```

## Parameters

| Param | Required | Description |
|-------|----------|-------------|
| open_id | Yes | Recipient's open_id (e.g. `ou_1d8ef0310c8508b6df8d5ddfce08c27c`) |
| message | Yes | Text content (text type) or JSON string (post/interactive) |
| msg_type | No | Message type: `text` (default), `post`, `interactive`, `image`, `file`, `audio`, `media`, `sticker` |

## Rate Limits

- 5 QPS per user (same recipient)
- 5 QPS per group
- 1000/min global

## Error Handling

The script exits with code 1 on failure and prints the error response. Common errors:

- `Arrearage` — Bot's Feishu account has billing issues
- `permission denied` — Bot lacks required scope or user is outside availability
- `invalid receive_id` — open_id not found or incorrect format

## Getting open_id

- From inbound message metadata (`sender_id` field)
- From the Feishu admin console
- Via the Feishu contact API
