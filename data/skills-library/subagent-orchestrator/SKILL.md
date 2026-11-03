---
name: subagent-orchestrator
description: "Master multi-agent orchestration skill for Antigravity. Enforces strict delegation mandate (never write feature code directly in orchestrator mode), provides pre-compiled native subagent profiles with embedded <skills> blocks, manages asynchronous lifecycle, and conducts final verification."
risk: low
source: antigravity-orchestrator-core (Apache 2.0)
date_added: 2026-10-31
---

# Subagent Orchestrator & Team Conductor

The **Subagent Orchestrator** skill governs end-to-end execution of large, complex, and multi-domain engineering tasks within Antigravity. It enables an agent to operate as a Chief Technical Lead and Multi-Agent Orchestrator, dynamically spawning, configuring, and coordinating worker subagents.

---

## ⛔ ИНВАРИАНТ ОБЯЗАТЕЛЬНОГО ДЕЛЕГИРОВАНИЯ (MANDATORY DISPATCH)

> [!IMPORTANT]
> **В режиме «Оркестратор» агенту КАТЕГОРИЧЕСКИ ЗАПРЕЩЕНО писать код реализации задач самостоятельно в основном диалоге.**
>
> Основной диалог принадлежит исключительно **Техлиду-Оркестратору**:
> 1. Если поступила задача на реализацию функционала — оркестратор **ОБЯЗАН** декомпозировать её и запустить специализированных субагентов через `define_subagent` и `invoke_subagent`.
> 2. Писать код через `write_file`/`edit_file` в основном диалоге допустимо **только** для сборки, разрешения конфликтов слияния и интеграционной склейки модулей.
> 3. Самостоятельное написание фичи в основном контексте вместо запуска субагентов расценивается как срыв роли оркестратора.

---

## 🧠 Как нативные субагенты Antigravity получают навыки

В Antigravity субагенты создаются нативным инструментом `define_subagent` и запускаются через `invoke_subagent`.
Чтобы субагент обладал полноценными предметными навыками (как в пресетах `backend`, `web`, `qa` и т.д.):

1. **Встраивание блока `<skills>` в `SystemPrompt`**:
   В `SystemPrompt` каждого субагента передаётся нативный блок `<skills>` с точными путями к `SKILL.md` в мастер-библиотеке (`C:/Users/Admin/.gemini/skills-library/<skill>/SKILL.md`).
2. **Нативное распознавание модели**:
   Базовая модель Antigravity (Gemini) обучена распознавать блок `<skills>` и автоматически открывает нужные файлы через `read_file` перед началом работы.
3. **Изоляция и параллелизм**:
   Субагенты не трогают файловые переключатели глобального пресета — каждый субагент изолирован и несёт свой набор навыков прямо в своём контексте.

---

## 🗂️ Каталог 8 предсобранных архетипов (`subagents_manifest.json`)

В файле `skills-library/subagent-orchestrator/subagents_manifest.json` (а также `~/.gemini/skills-library/subagent-orchestrator/subagents_manifest.json`) уже скомпилированы готовые профили:

| Имя типа (`TypeName`) | Роль | Встроенный набор навыков |
|---|---|---|
| `subagent-backend` | Senior Backend & Database Engineer | `database-design`, `api-design-principles`, `nodejs-best-practices`, `go-concurrency-patterns`, `postgres-dba-optimizer`, `fastapi-async-architect`, `go-highload-services`, `rust-systems-safe`, `redis-distributed-caching`, `kafka-event-streaming` |
| `subagent-web` | Senior Frontend & Modern Web Engineer | `ui-ux-pro-max`, `frontend-design`, `react-best-practices`, `seo-audit`, `form-cro`, `nextjs-fullstack-guru`, `tailwind-v4-master`, `react-performance-opt`, `vue-nuxt-expert`, `web-state-query-master`, `web-performance-auditor` |
| `subagent-qa` | Senior QA Automation & Test Engineer | `playwright-e2e-guru`, `windows-desktop-control`, `test-driven-development`, `test-fixing`, `code-review-checklist`, `unit-test-synthesizer`, `k6-load-testing-pro`, `api-contract-tester` |
| `subagent-devops` | Senior DevOps & Cloud Infrastructure Engineer | `docker-container-master`, `k8s-manifest-generator`, `github-actions-ci-cd`, `terraform-iac-architect`, `incident-responder`, `deployment-procedures`, `aws-serverless`, `observability-telemetry-guru` |
| `subagent-appsec` | Senior Application Security & Reverse Engineer | `top-web-vulnerabilities`, `api-security-best-practices`, `ethical-hacking-methodology`, `appsec-code-auditor`, `reverse-engineer-electron`, `win32-powershell-automation`, `windows-desktop-control`, `network-traffic-intercept`, `api-pentest-fuzzer`, `premortem-redteam` |
| `subagent-product` | Product Partner & Monetization Strategist | `product-monetization-strategy`, `market-competitor-intelligence`, `ai-product-partner`, `browser`, `search-specialist`, `deep-research-synthesizer`, `replica-entrepreneur`, `replica-brand`, `replica-launch`, `form-cro`, `ui-ux-pro-max`, `seo-audit` |
| `subagent-research` | Deep Web Researcher & OSINT Analyst | `search-specialist`, `deep-research-synthesizer`, `web-scraper-harvester`, `osint-recon-investigator` |
| `subagent-reverse` | Clean-Room Reverse Engineering Specialist | 11 скиллов `replica-*` + `reverse-engineer-electron` |

---

## ⚡ Рецепт запуска субагента: 3 шага

### Шаг 1: Прочитать манифест или скомпилировать профиль
Оркестратор может прочитать готовый манифест через `read_file`:
`skills-library/subagent-orchestrator/subagents_manifest.json`  
(или вызвать фабрику: `python skills-library/subagent-orchestrator/scripts/subagent_factory.py <preset>`).

### Шаг 2: Зарегистрировать субагента (`define_subagent`)
```javascript
define_subagent({
  Name: "subagent-backend",
  Description: "Specialized Backend subagent equipped with PostgreSQL, database design, FastAPI, Node.js, Go, Redis, and API principles skills",
  EnableWriteTools: true,
  EnableMCPTools: true,
  EnableSubagentTools: false, // Защита от неконтролируемой рекурсии
  SystemPrompt: manifest["backend"]["SystemPrompt"]
});
```

### Шаг 3: Запустить выполнение задачи (`invoke_subagent`)
```javascript
invoke_subagent({
  TypeName: "subagent-backend",
  Role: "Senior Backend & Database Engineer",
  Workspace: "inherit", // или "branch" при параллельном редактировании кода
  Model: "inherit",
  Prompt: "Разработай слой доступа к данным и схему таблиц для авторизации. Обрати внимание на навыки в <skills>."
});
```

#### ⚡ Параллельный веерный запуск (Concurrent Multi-Agent Fan-Out)
Если этап или задача содержит независимые потоки работ (например, Backend API + Frontend UI, либо DevOps Action + QA Тестовый стенд), оркестратор **вызывает несколько `invoke_subagent` одновременно в рамках ОДНОГО хода**:
```javascript
// Поток 1: DevOps специалист
invoke_subagent({
  TypeName: "subagent-devops",
  Role: "GitHub Actions & Delivery Engineer",
  Workspace: "branch",
  Prompt: "Реализуй Rich Step Summary и артефакты патчей в src/autoheal/delivery/..."
});

// Поток 2: QA & Benchmark специалист
invoke_subagent({
  TypeName: "subagent-qa",
  Role: "QA Benchmark Engineer",
  Workspace: "branch",
  Prompt: "Создай синтетический тестовый стенд на 15 багов в tests/test_benchmarks.py..."
});
```

После вызова `invoke_subagent` оркестратор **завершает свой ход** (yield turn). Система автоматически разбудит оркестратора, когда субагенты пришлют отчёты!

---

## 📢 Уведомления о вехах
Когда субагент возвращает сообщение с результатами, оркестратор сообщает пользователю:
`📢 [Этап завершен: subagent-backend] Создана схема базы данных и миграции. Запускаем subagent-qa для E2E тестов.`
