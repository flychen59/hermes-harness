#!/usr/bin/env bash
# Send a Feishu private message via API
# Usage: send_message.sh <open_id> <message_text> [msg_type]
#   open_id    - Recipient's open_id (e.g. ou_xxx)
#   message_text - Message content (for text: plain text; for post: JSON string)
#   msg_type   - Message type: text (default), post, interactive, image, file, etc.

set -euo pipefail

OPEN_ID="${1:?Usage: send_message.sh <open_id> <message_text> [msg_type]}"
MSG_TEXT="${2:?Message text required}"
MSG_TYPE="${3:-text}"

# Read app credentials from openclaw config
CONFIG_FILE="$HOME/.openclaw/openclaw.json"
if [[ ! -f "$CONFIG_FILE" ]]; then
  echo "ERROR: openclaw.json not found" >&2
  exit 1
fi

APP_ID=$(jq -r '.channels.feishu.accounts.default.appId // empty' "$CONFIG_FILE")
APP_SECRET=$(jq -r '.channels.feishu.accounts.default.appSecret // empty' "$CONFIG_FILE")

if [[ -z "$APP_ID" || -z "$APP_SECRET" ]]; then
  echo "ERROR: Feishu appId/appSecret not configured" >&2
  exit 1
fi

# Get tenant_access_token
TOKEN_RESP=$(curl -s -X POST "https://open.feishu.cn/open-apis/auth/v3/tenant_access_token/internal" \
  -H "Content-Type: application/json" \
  -d "{\"app_id\":\"$APP_ID\",\"app_secret\":\"$APP_SECRET\"}")

TOKEN=$(echo "$TOKEN_RESP" | jq -r '.tenant_access_token // empty')
if [[ -z "$TOKEN" ]]; then
  echo "ERROR: Failed to get tenant_access_token: $TOKEN_RESP" >&2
  exit 1
fi

# Build content based on msg_type
if [[ "$MSG_TYPE" == "text" ]]; then
  # Escape JSON special characters in text
  ESCAPED_TEXT=$(echo "$MSG_TEXT" | jq -Rs .)
  CONTENT="{\"text\":$ESCAPED_TEXT}"
elif [[ "$MSG_TYPE" == "interactive" ]]; then
  # For card messages, pass raw JSON
  CONTENT="$MSG_TEXT"
else
  # For other types, pass as-is
  CONTENT="$MSG_TEXT"
fi

# Send message
SEND_RESP=$(curl -s -X POST "https://open.feishu.cn/open-apis/im/v1/messages?receive_id_type=open_id" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "{\"receive_id\":\"$OPEN_ID\",\"msg_type\":\"$MSG_TYPE\",\"content\":$(echo "$CONTENT" | jq -Rs .)}")

CODE=$(echo "$SEND_RESP" | jq -r '.code // "unknown"')
if [[ "$CODE" != "0" ]]; then
  echo "ERROR: Send failed (code=$CODE): $SEND_RESP" >&2
  exit 1
fi

MSG_ID=$(echo "$SEND_RESP" | jq -r '.data.message_id // "unknown"')
CHAT_ID=$(echo "$SEND_RESP" | jq -r '.data.chat_id // "unknown"')
echo "OK message_id=$MSG_ID chat_id=$CHAT_ID"
