# 多轮自适应 Skill 框架 + 多 Agent 共享进化方案

## 目标

在 Hermes Harness 上实现：
1. **多轮对话中快速适配用户需求** — 根据对话上下文自动选择/组合/生成 Skill
2. **Skill 可进化** — 基于 trajectory 反馈自动优化 Skill prompt
3. **多 Agent 共享进化** — Hermes / OpenClaw / 其他 Agent 共享 Skill 库、经验和记忆

---

## 架构设计

```
┌─────────────────────────────────────────────────────┐
│                 Shared Skill Layer                    │
│  ~/bot-shared/skills/  (Git-managed skill registry)  │
│  ├── registry.json        # Skill 元数据 + 评分      │
│  ├── evolved/             # 进化后的 skills           │
│  ├── templates/           # Skill 模板                │
│  └── feedback/            # 使用反馈 + trajectory     │
├─────────────────────────────────────────────────────┤
│              Skill Orchestrator (Core)                │
│  1. Intent Detector     — 从用户消息提取意图          │
│  2. Skill Matcher       — 匹配已有 Skill              │
│  3. Skill Composer      — 组合多个 Skill              │
│  4. Skill Generator     — 无匹配时动态生成新 Skill     │
│  5. Evolution Engine    — 基于 feedback 优化 Skill    │
├─────────────────────────────────────────────────────┤
│              Multi-Agent Sync Layer                   │
│  ~/bot-shared/protocol/                               │
│  ├── skill-sync.md       # Skill 同步协议             │
│  ├── evolution-log.md    # 进化日志                   │
│  └── agent-profiles/     # 每个 Agent 的能力画像      │
└─────────────────────────────────────────────────────┘
```

---

## Phase 1: Skill Registry + Intent Matching

### 1.1 统一 Skill Registry

在 `~/bot-shared/skills/registry.json` 维护一个共享 registry：

```json
{
  "version": "1.0",
  "skills": [
    {
      "id": "github-trending",
      "name": "GitHub Trending 分析",
      "triggers": ["github", "top star", "热门项目", "trending"],
      "category": "research",
      "agents": ["hermes", "openclaw"],
      "rating": 4.5,
      "usage_count": 23,
      "last_used": "2026-05-22",
      "skill_path": "skills/research/github-trending/SKILL.md",
      "evolved_from": null,
      "version": 1
    }
  ]
}
```

### 1.2 意图检测 + Skill 匹配

在 Harness 的 `prompt_builder.py` 中增加 Skill 路由逻辑：

```python
# agent/skill_router.py
class SkillRouter:
    def __init__(self, registry_path="~/bot-shared/skills/registry.json"):
        self.registry = self._load_registry(registry_path)
    
    def match(self, user_message: str, context: dict) -> list[SkillMatch]:
        """从用户消息匹配最佳 Skill"""
        # 1. 关键词匹配 triggers
        # 2. 语义相似度匹配（embedding）
        # 3. 上下文感知：对话历史 + 用户偏好
        # 4. 返回 top-k 匹配，按 rating * usage_count 排序
        pass
    
    def compose(self, skills: list[SkillMatch]) -> str:
        """组合多个 Skill 的 prompt"""
        pass
```

### 1.3 集成到 Hermes Gateway

在 `run_agent.py` 的 turn 循环中插入：

```python
# 用户消息 → Skill 路由 → 注入 Skill prompt → Agent 执行
skill_matches = skill_router.match(user_message, context)
if skill_matches:
    skill_prompt = skill_router.compose(skill_matches)
    system_prompt += f"\n\n[Active Skills]\n{skill_prompt}"
```

---

## Phase 2: 动态 Skill 生成

当 registry 中没有匹配的 Skill 时，动态生成：

```python
# agent/skill_generator.py
class SkillGenerator:
    async def generate(self, user_need: str, context: dict) -> Skill:
        """根据用户需求动态生成新 Skill"""
        prompt = f"""
        用户需要一个新的 Skill 来处理以下需求：
        {user_need}
        
        对话上下文：
        {context['recent_messages']}
        
        请生成一个 SKILL.md 文件，包含：
        1. name + description
        2. triggers（关键词列表）
        3. 具体执行步骤
        4. 预期输出格式
        """
        skill_content = await llm.generate(prompt)
        # 保存到 ~/bot-shared/skills/generated/
        # 注册到 registry
        return skill
```

---

## Phase 3: Skill 进化引擎

### 3.1 反馈收集

每次 Skill 使用后收集：
- 用户满意度（显式/隐式反馈）
- Token 消耗
- 完成率
- 错误类型

```json
// ~/bot-shared/skills/feedback/github-trending.jsonl
{"timestamp":"2026-05-22","agent":"hermes","rating":4,"tokens":1200,"success":true,"feedback":"结果准确但缺少中文项目"}
```

### 3.2 自动进化

当 feedback 积累到阈值（如 5 次使用），触发进化：

```python
class SkillEvolver:
    async def evolve(self, skill_id: str) -> Skill:
        feedback = load_feedback(skill_id)
        current_skill = load_skill(skill_id)
        
        evolution_prompt = f"""
        当前 Skill：
        {current_skill.content}
        
        使用反馈（最近 {len(feedback)} 次）：
        {format_feedback(feedback)}
        
        平均评分：{avg_rating}/5
        主要问题：{extract_issues(feedback)}
        
        请优化这个 Skill，解决上述问题。
        只修改需要改进的部分，保持已有的有效逻辑。
        """
        
        evolved_content = await llm.generate(evolution_prompt)
        # 保存为 v2，保留 v1 作为回退
        save_skill(skill_id, evolved_content, version=current_skill.version + 1)
```

---

## Phase 4: 多 Agent 共享进化

### 4.1 共享协议

在 `~/bot-shared/protocol/skill-sync.md` 定义协议：

```markdown
# Skill Sync Protocol v1

## 注册新 Skill
任何 Agent 创建新 Skill 后，写入 ~/bot-shared/skills/ 并更新 registry.json

## 进化通知
Agent 完成进化后，写入 ~/bot-shared/protocol/evolution-log.md：
- 源 Agent
- Skill ID + 新版本
- 改进了什么
- 对其他 Agent 的兼容性影响

## 能力广播
每个 Agent 启动时写入 ~/bot-shared/protocol/agent-profiles/<agent>.json：
- 可用 Skills
- 擅长领域
- 当前模型 + 能力
```

### 4.2 跨 Agent Skill 转移

```python
class CrossAgentSkillTransfer:
    def sync_from_peer(self, agent_name: str):
        """从其他 Agent 同步进化的 Skills"""
        # 读取 ~/bot-shared/skills/evolved/ 中的新版本
        # 对比本地版本
        # 如果远程版本更高且 rating 更好，自动升级
        pass
    
    def broadcast_evolution(self, skill_id: str, new_version: int):
        """广播 Skill 进化"""
        # 写入 evolution-log
        # 通知其他 Agent（通过 bot-shared/messages/）
        pass
```

---

## 实施计划

| Phase | 内容 | 时间 | 优先级 |
|-------|------|------|--------|
| **P1** | Skill Registry + Intent Matcher | 2-3 天 | 🔴 高 |
| **P2** | 动态 Skill 生成 | 1-2 天 | 🔴 高 |
| **P3** | 进化引擎（反馈收集 + 自动优化） | 3-5 天 | 🟡 中 |
| **P4** | 多 Agent 共享 + 跨 Agent 转移 | 2-3 天 | 🟡 中 |

### 快速启动（今天就能跑）

1. **创建 `~/bot-shared/skills/` 目录结构**
2. **实现 `SkillRouter`** — 基于关键词匹配的 v1
3. **写 3-5 个高频 Skill** — 作为种子（github-trending, paper-search, agent-news 等）
4. **集成到 Hermes Gateway** — 在 turn 循环中注入

---

## 与现有架构的对接点

- **Hermes Harness**: `agent/skill_commands.py` + `agent/skill_preprocessing.py` — 已有 Skill 加载机制
- **Memory Manager**: `agent/memory_manager.py` — 共享记忆可复用
- **Trajectory**: `agent/trajectory.py` — 进化的训练数据来源
- **Bot Shared Protocol**: `~/bot-shared/` — 已有跨 Bot 通信基础设施
- **OpenClaw Skills**: `~/.openclaw/workspace/skills/` — 可直接复用 OpenClaw 的 Skill 格式

---

## 参考项目

| 项目 | Star | 借鉴点 |
|------|------|--------|
| obra/superpowers | 202K | Agentic Skills 框架设计 |
| anthropics/skills | 139K | Skill 格式和分类 |
| bytedance/deer-flow | 69K | 多 Agent 协作 |
| VoltAgent/awesome-openclaw-skills | 49K | Skill 生态 |
| evilsocket/audit | 428 | 多阶段 Agent Skill |
