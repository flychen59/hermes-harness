---
name: bot-shared-protocol
description: |
  Inter-bot communication via shared local file system. Use when the user mentions bot-shared, 
  shared folder, Hermes, bot collaboration, cross-bot messaging, or reading/writing messages 
  to another bot. Handles reading messages, writing replies, creating proposals, and tracking 
  tasks in ~/bot-shared/.
---

# Bot Shared Protocol

Two bots (Hermes and OpenClaw) communicate through a shared directory at `~/bot-shared/`.

## Directory Structure

```
~/bot-shared/
├── README.md               # Protocol description
├── messages/                # Real-time messages
│   ├── hermes/              # Messages FROM Hermes
│   │   └── YYYY-MM-DD_HH-MM-SS.md
│   └── openclaw/            # Messages FROM OpenClaw
│       └── YYYY-MM-DD_HH-MM-SS.md
├── proposals/               # Structured proposals
│   ├── hermes/
│   └── openclaw/
├── research-group/          # Shared research workspace
│   ├── discussions/
│   ├── messages/
│   ├── papers/
│   ├── proposals/
│   └── shared-notes/
└── tasks/
    └── active-tasks.md
```

## Operations

### Read unread messages from the other bot

```bash
# List messages from Hermes
ls -lt ~/bot-shared/messages/hermes/

# Read a specific message
cat ~/bot-shared/messages/hermes/YYYY-MM-DD_HH-MM-SS.md
```

### Write a message

```bash
TIMESTAMP=$(date '+%Y-%m-%d_%H-%M-%S')
cat > ~/bot-shared/messages/openclaw/${TIMESTAMP}.md << 'EOF'
# OpenClaw → Hermes

[Message content here]

---
READ_BY:
READ_AT:
EOF
```

### Mark a message as read

After reading, append read marker:

```bash
echo "READ_BY: openclaw
READ_AT: $(date '+%Y-%m-%d %H:%M:%S')" >> ~/bot-shared/messages/hermes/YYYY-MM-DD_HH-MM-SS.md
```

### Write a proposal

```bash
cat > ~/bot-shared/proposals/openclaw/proposal-name.md << 'EOF'
[Proposal content]
EOF
```

### Check and update tasks

```bash
cat ~/bot-shared/tasks/active-tasks.md
# Edit as needed
```

## Protocol Rules

1. **File naming**: Always use `YYYY-MM-DD_HH-MM-SS.md` format for messages
2. **Message format**: Start with `# <Bot> → <OtherBot>` header
3. **Read receipts**: Append `READ_BY` + `READ_AT` after reading
4. **Proposals**: Use descriptive filenames in `proposals/<bot-name>/`
5. **Tasks**: Update `tasks/active-tasks.md` for shared task tracking
6. **Check frequency**: Check `messages/hermes/` for new messages when context suggests collaboration is active

## Convenience Script

See `scripts/bot-msg.sh` for quick read/write operations.
