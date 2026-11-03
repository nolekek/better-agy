---
name: browser
description: "Native Antigravity browser automation skill using Chrome DevTools MCP. Controls Chrome browser to navigate, inspect, take snapshots, click, fill forms, evaluate scripts, upload files, and automate web tasks."
---

# Native Browser Automation (Chrome DevTools MCP)

Встроенный в Antigravity базовый навык браузерной автоматизации. Управляет браузером Chrome через протокол Chrome DevTools MCP и CLI `chrome-devtools`.

---

## ⚡ Быстрый старт и проверка состояния

CLI `chrome-devtools` встроен в систему и доступен из любой командной строки. Демон запускается неявно при первом обращении или вручную:

```bash
# Проверка статуса демона
chrome-devtools status

# Запуск браузера (с отображением UI для сессий и авторизаций):
chrome-devtools start --headless=false

# Либо подключение к уже запущенному браузеру пользователя:
chrome-devtools start --browserUrl http://127.0.0.1:9222
```

---

## 🧭 Базовый рабочий цикл (Workflow)

1. **Список страниц**: `chrome-devtools list_pages` (получаем `pageId`, например `1`).
2. **Создание / переход**:
   ```bash
   chrome-devtools new_page "https://example.com"
   # или для существующей вкладки:
   chrome-devtools navigate_page 1 --url "https://example.com"
   ```
3. **Ожидание готовности**: `chrome-devtools wait_for 1 --selector "css-selector" --timeout 5000`
4. **Снимок элементов (Snapshot)**:
   ```bash
   chrome-devtools take_snapshot 1
   ```
   Снимок строит дерево доступности с уникальными идентификаторами элементов `uid`:
   ```text
   uid=1_1 heading "Example Domain" level="1"
   uid=1_5 button "Войти"
   uid=1_8 textbox "Поиск"
   ```
5. **Действия с элементами по их `uid`**:
   ```bash
   # Клик по кнопке или ссылке
   chrome-devtools click 1 "1_5"

   # Ввод текста в поле
   chrome-devtools fill 1 "1_8" "Поисковый запрос"

   # Нажатие клавиш
   chrome-devtools press_key 1 "Enter"

   # Загрузка файла через input[type=file]
   chrome-devtools upload_file 1 "1_12" "C:/path/to/file.zip"
   ```
6. **Выполнение JavaScript**:
   ```bash
   chrome-devtools evaluate_script "() => document.title" --pageId 1
   ```
7. **Скриншот страницы**:
   ```bash
   chrome-devtools take_screenshot 1 --filePath "screenshot.png"
   ```

---

## 🎯 Правила надежности:

- **Всегда делайте свежий snapshot перед кликом или вводом**: если страница обновилась или сработал AJAX, старые `uid` становятся недействительными.
- **Предпочитайте snapshot скриншотам**: текстовый снимок страницы работает в десятки раз быстрее и точнее определяет элементы интерфейса.
- **Управление сессиями и профилями**: чтобы сохранять cookies и авторизацию между перезапусками, используйте единый каталог данных:
  `--userDataDir "%USERPROFILE%/.cache/chrome-devtools-mcp/chrome-profile"`
