# 手机端 Skill 快捷控制面板 — 整合方案

## 现状

Harness 已有 Web Dashboard（`hermes web`，端口 9119）：
- ✅ SkillsPage — Skill 列表 + 开关
- ✅ ChatPage — xterm 终端聊天
- ✅ ConfigPage — 配置管理
- ✅ CronPage — 定时任务
- ❌ **没有移动端适配的快捷操作面板**
- ❌ **没有"一键发送指令"的按钮式 UI**

## 方案：在 Hermes Web Dashboard 上增加 Mobile Quick Panel

### 1. 新增 `/quick` 页面（Mobile-First）

```
┌─────────────────────────────┐
│  🔧 Hermes Quick Panel      │
├─────────────────────────────┤
│                             │
│  ⭐ 常用 Skill（一键切换）    │
│  ┌─────┐ ┌─────┐ ┌─────┐  │
│  │ 🔍  │ │ 📄  │ │ 💻  │  │
│  │GitHub│ │论文  │ │ 代码 │  │
│  │Trend │ │搜索  │ │审查  │  │
│  └─────┘ └─────┘ └─────┘  │
│  ┌─────┐ ┌─────┐ ┌─────┐  │
│  │ 📊  │ │ 🔧  │ │ 🤖  │  │
│  │分析 │ │调试  │ │多Agent│  │
│  └─────┘ └─────┘ └─────┘  │
│                             │
│  📋 快捷指令（一键发送）      │
│  ┌─────────────────────────┐│
│  │ 📰 今日 AI 新闻速览      ││
│  │ ⭐ GitHub 本周热门        ││
│  │ 📊 我的 Agent 使用统计    ││
│  │ 🔄 重启所有服务           ││
│  │ 📡 检查所有 Agent 状态    ││
│  └─────────────────────────┘│
│                             │
│  💬 快速对话                 │
│  ┌─────────────────────────┐│
│  │ 输入指令...          ➤  ││
│  └─────────────────────────┘│
└─────────────────────────────┘
```

### 2. 实现步骤

#### Step 1: 新增 `QuickPanelPage.tsx`

复用现有 API：
- `api.getSkills()` → 显示 Skill 开关网格
- `api.toggleSkill()` → 一键开关
- 新增 `/api/quick-command` → 快捷指令发送到当前会话

#### Step 2: 新增后端 API

在 `web_server.py` 中新增：

```python
@app.post("/api/quick-command")
async def quick_command(body: QuickCommand):
    """从快捷面板发送指令到活跃会话"""
    # 找到当前活跃的 gateway session
    # 注入消息到对话队列
    # 返回执行结果
    pass

@app.get("/api/agents/status")
async def agents_status():
    """返回所有 Agent 状态"""
    # Hermes: gateway_state.json
    # OpenClaw: openclaw gateway status
    pass
```

#### Step 3: PWA 支持（手机添加到主屏幕）

在 `index.html` 中添加：
- `manifest.json`（PWA manifest）
- Service Worker（离线缓存）
- Apple touch icon

#### Step 4: 响应式布局

已有 shadcn/ui 组件（Button, Card, Switch 等），直接用：
- 底部 Tab 导航（移动端）
- 卡片式 Skill 网格
- 大按钮（方便手指点击）

### 3. 文件清单

需要新增/修改的文件：

```
web/src/
├── pages/
│   └── QuickPanelPage.tsx     # 新增：快捷面板页
├── components/
│   ├── SkillGrid.tsx          # 新增：Skill 网格组件
│   ├── QuickCommands.tsx      # 新增：快捷指令列表
│   └── QuickChat.tsx          # 新增：快速对话输入
├── lib/
│   └── api.ts                 # 修改：新增 quick-command API
└── App.tsx                    # 修改：新增 /quick 路由

web/public/
├── manifest.json              # 新增：PWA manifest
└── icons/                     # 新增：PWA 图标

hermes_cli/web_server.py       # 修改：新增 API 端点
```

### 4. 替代方案：Telegram Bot 快捷按钮

如果不想改 Web UI，也可以用 Telegram Inline Keyboard：

```python
# 用户在 Telegram 发 /quick
keyboard = [
    [InlineKeyboardButton("⭐ GitHub Trending", callback_data="skill:github-trending")],
    [InlineKeyboardButton("📄 论文搜索", callback_data="skill:arxiv")],
    [InlineKeyboardButton("🔄 重启服务", callback_data="cmd:restart")],
    [InlineKeyboardButton("📡 Agent 状态", callback_data="cmd:status")],
]
```

但飞书的 Interactive Card 也能实现同样效果，而且你已经在用飞书了。

### 5. 飞书方案（最快实现）

通过飞书消息卡片的交互按钮实现：

```json
{
  "config": { "wide_screen_mode": true },
  "header": { "title": { "tag": "plain_text", "content": "🔧 Hermes Quick Panel" } },
  "elements": [
    {
      "tag": "action",
      "actions": [
        { "tag": "button", "text": { "tag": "plain_text", "content": "⭐ GitHub热门" }, "value": {"action": "github-trending"} },
        { "tag": "button", "text": { "tag": "plain_text", "content": "📄 论文搜索" }, "value": {"action": "arxiv"} },
        { "tag": "button", "text": { "tag": "plain_text", "content": "📡 状态" }, "value": {"action": "status"} }
      ]
    }
  ]
}
```

**优点**：零代码部署，直接在飞书群里点击按钮
**缺点**：受飞书卡片限制

---

## 推荐方案

| 方案 | 实现时间 | 体验 |
|------|---------|------|
| **飞书卡片按钮** | 1-2 小时 | ⭐⭐⭐ 手机直接用 |
| **Telegram Inline Keyboard** | 1-2 小时 | ⭐⭐⭐ 手机直接用 |
| **Web Quick Panel** | 1-2 天 | ⭐⭐⭐⭐⭐ 最完整 |
| **PWA 应用** | 2-3 天 | ⭐⭐⭐⭐⭐ 像原生 App |

**建议**：先上飞书卡片按钮（最快），再迭代到 Web Quick Panel。
