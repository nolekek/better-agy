---
name: antigravity-core-dev
description: "Master internal development & maintenance skill for Antigravity core engineers. Governs Electron ASAR unpacking and repacking, multi-account rotation bridge, skills-library synchronization, credential vault testing, and full-cycle release automation."
risk: low
source: antigravity-core-team (Apache 2.0)
date_added: 2026-11-01
---

# Antigravity Core Engineer & Self-Evolution Runbook

The **Antigravity Core Dev** skill is the authoritative operating manual for maintaining, debugging, extending, and testing the Antigravity desktop application, its Multi-Account carousel, ASAR bundle, and Dynamic Preset Control Center.

---

## ⚡ One-Click Core Automation CLI (`scripts/antigravity_dev.py`)

Always use the native core developer tool to execute routine maintenance operations:

```bash
# 1. Проверить синтаксис всех ключевых JS-файлов (root + dist):
python scripts/antigravity_dev.py check

# 2. Синхронизировать измененные файлы из корня в asar_extracted_228/dist/:
python scripts/antigravity_dev.py sync

# 3. Синхронизировать навыки с ~/.gemini/ и обновить skills_library.tar.gz:
python scripts/antigravity_dev.py sync-skills

# 4. Собрать app.asar (с --unpack правилами), развернуть в клиент и сжать payload/app.asar.gz:
python scripts/antigravity_dev.py repack

# 5. Запустить полный мастер-тест раннер (79/79 тестов):
python scripts/antigravity_dev.py test

# 6. Полный цикл под ключ (check -> sync -> sync-skills -> repack -> test):
python scripts/antigravity_dev.py full-cycle
```

---

## 🏛️ Архитектурные правила и инварианты ядра

### 1. Правило распаковки ASAR (`--unpack`)
ASAR-архив упаковывается строго с сохранением нативных и внешних исполняемых файлов вне архива:
```bash
npx asar pack asar_extracted_228 app.asar.patched_master --unpack "{**/chrome-devtools-mcp/**,**/credHelper.py}"
```
* **Зачем:** Python `credHelper.py` и исполняемые бинарники Chrome DevTools не могут исполняться напрямую из виртуальной файловой системы ASAR без извлечения.

### 2. Синхронизация файлов исходников
При любых изменениях в кодовой базе проекта:
- `skillManager.js`
- `preload.js`
- `stackDetector.js`
- `multiaccManager.js`
- `credHelper.py`
Файлы **ОБЯЗАНЫ быть синхронизированы** в `asar_extracted_228/dist/` (и `asar_extracted_228/credHelper.py`) перед сборкой ASAR. Всегда запускайте `python scripts/antigravity_dev.py sync`.

### 3. Тестовый инвариант 100% PASS (79/79)
Перед любым коммитом или релизом:
```bash
node tests/run_all.js
```
Тест-сьют охватывает 4 уровня (R1-R4, краевые случаи, комбинации состояний, боевые сценарии ротации и Python Win32 Credential Vault). Ни один тест не должен падать.

### 4. PowerShell Invariants в среде Windows
- **Спецсимволы в строках**: Символы `&`, `<`, `>`, `|` внутри параметров `-e` или строк PowerShell приводят к `AmpersandNotAllowed` / `ParserError`. Для сложных манипуляций всегда используйте вспомогательные скрипты в `<appDataDir>\brain\<conversation-id>\scratch\`.
- **Переменные окружения**: Вместо `%USERPROFILE%` в PowerShell используется `$env:USERPROFILE`.

---

## 🧭 Матрица ключевых файлов Antigravity

| Компонент | Расположение | Назначение |
|---|---|---|
| **Диспетчер пресетов** | `skillManager.js` | Реестр `PRESETS`, переключение пресетов, подсчет токенов, `ALWAYS_ACTIVE` |
| **UI Control Center** | `preload.js` | Рендеринг боковой шторки, карточек пресетов и баннеров в окне Electron |
| **Детектор стека** | `stackDetector.js` | Автоопределение технологий проекта и рекомендация пресета |
| **Ротация аккаунтов** | `multiaccManager.js` | Перехват OAuth, 4-слотовая карусель, failover по квотам, debounce |
| **Win32 Vault Helper** | `credHelper.py` | Шифрование DPAPI, чтение/запись Windows Credential Vault, упаковка JWT |
| **Языковой сервер** | `languageServer.js` | Запуск и мониторинг бинарника `language_server.exe` с флагом `ANTIGRAVITY_VSCODE_HOST=1` |
| **Скрипт переключения** | `scripts/switch.py` | Двухуровневый свитчер (HTTP IPC Bridge `51122` $\to$ тогл файлов) |
| **Фабрика субагентов** | `skills-library/subagent-orchestrator/scripts/subagent_factory.py` | Генератор манифеста субагентов со скомпилированными блоками `<skills>` |
| **Инсталлятор** | `build_installer/installer.py` | Автономный установщик мода поверх оригинальной Antigravity |
