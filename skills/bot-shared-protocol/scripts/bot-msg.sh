#!/bin/bash
# Bot Shared Protocol - Quick message operations
# Usage:
#   bot-msg.sh read [hermes|openclaw]          - List unread messages from specified bot
#   bot-msg.sh show <filename> [hermes|openclaw] - Show a specific message
#   bot-msg.sh mark-read <filename> <bot>      - Mark message as read by openclaw
#   bot-msg.sh write <message-content>         - Write a new message to openclaw outbound

SHARED_DIR="$HOME/bot-shared"
MY_NAME="openclaw"
OTHER_BOT="hermes"

case "${1}" in
  read)
    BOT="${2:-$OTHER_BOT}"
    echo "=== Messages from $BOT ==="
    ls -lt "$SHARED_DIR/messages/$BOT/" 2>/dev/null | grep '.md$'
    ;;
  show)
    FILE="$2"
    BOT="${3:-$OTHER_BOT}"
    cat "$SHARED_DIR/messages/$BOT/$FILE" 2>/dev/null
    ;;
  mark-read)
    FILE="$2"
    BOT="${3:-$OTHER_BOT}"
    echo "READ_BY: $MY_NAME
READ_AT: $(date '+%Y-%m-%d %H:%M:%S')" >> "$SHARED_DIR/messages/$BOT/$FILE"
    echo "Marked $FILE as read."
    ;;
  write)
    shift
    MSG="$*"
    TIMESTAMP=$(date '+%Y-%m-%d_%H-%M-%S')
    cat > "$SHARED_DIR/messages/$MY_NAME/${TIMESTAMP}.md" << EOF
# OpenClaw → Hermes

$MSG

---
READ_BY:
READ_AT:
EOF
    echo "Message written to messages/$MY_NAME/${TIMESTAMP}.md"
    ;;
  *)
    echo "Usage: bot-msg.sh {read|show|mark-read|write} [args...]"
    echo "  read [bot]              - List messages from bot (default: hermes)"
    echo "  show <file> [bot]       - Show a specific message"
    echo "  mark-read <file> [bot]  - Mark message as read"
    echo "  write <content>         - Write new message"
    ;;
esac
