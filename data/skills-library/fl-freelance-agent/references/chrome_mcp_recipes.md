# Chrome DevTools MCP Recipes for FL.ru Automation

Этот справочник описывает точные цепочки вызовов инструментов `chrome-devtools-mcp` для сценариев работы агента на платформе FL.ru.

---

## Рецепт 1: Инициализация и переход в ленту заказов

```javascript
// 1. Проверяем открытые страницы браузера
list_pages()

// 2. Если страницы нет, создаем новую с URL ленты проектов FL.ru
new_page(url="https://www.fl.ru/projects/?kind=5")
// -> возвращает pageId (например, 1)

// 3. Либо переходим на существующей странице
navigate_page(pageId=1, url="https://www.fl.ru/projects/?kind=5")

// 4. Ожидаем появления списка проектов
wait_for(pageId=1, selector=".b-post, .projects-list, [data-project-id]", timeout=8000)

// 5. Получаем структурированный снимок страницы
take_snapshot(pageId=1)
```

---

## Рецепт 2: Чтение карточки проекта и оценка ТЗ

```javascript
// 1. Переход на страницу конкретного проекта
navigate_page(pageId=1, url="https://www.fl.ru/projects/5432101/razrabotat-parser.html")

// 2. Снимок структуры страницы (текст задания, прикрепленные файлы, контакты)
take_snapshot(pageId=1)

// 3. При необходимости извлечь текст описания через evaluate_script
evaluate_script(pageId=1, functionDeclaration="""
() => {
  const descEl = document.querySelector('.b-project__txt, [id^="project-description"], .project-body');
  const files = Array.from(document.querySelectorAll('a[href*="/upload/projects/"]')).map(a => ({
    name: a.innerText.trim(),
    url: a.href
  }));
  return {
    description: descEl ? descEl.innerText.trim() : '',
    files: files
  };
}
""")
```

---

## Рецепт 3: Заполнение и отправка отклика (Bid)

```javascript
// 1. Делаем свежий снимок для получения актуальных uid элементов формы
take_snapshot(pageId=1)

// Предположим, snapshot вернул:
// uid=1_42 button "Ответить на проект"
// uid=1_55 textarea "Текст отклика"
// uid=1_60 input "Стоимость"
// uid=1_62 input "Срок (дней)"
// uid=1_70 input "Прикрепить файл" [type=file]
// uid=1_75 button "Отправить"

// 2. Если форма свернута, кликаем на раскрытие
click(pageId=1, uid="1_42")

// 3. Заполняем текст заявки
fill(pageId=1, uid="1_55", value="Здравствуйте! Изучил ТЗ...")

// 4. Заполняем стоимость и срок
fill(pageId=1, uid="1_60", value="4000")
fill(pageId=1, uid="1_62", value="1")

// 5. Опционально: прикрепляем микро-демо или архитектурную схему
upload_file(pageId=1, uid="1_70", filePath="C:/Users/Admin/.../demo_script.py")

// 6. Нажимаем кнопку отправки
click(pageId=1, uid="1_75")
```

---

## Рецепт 4: Мониторинг входящих сообщений в чате (`/messages/`)

```javascript
// 1. Переход в раздел личных сообщений
navigate_page(pageId=1, url="https://www.fl.ru/messages/")

// 2. Ожидание загрузки списка диалогов
wait_for(pageId=1, selector=".b-chat, .dialogs-list, [data-dialog-id]", timeout=5000)

// 3. Получение снимка для проверки непрочитанных сообщений
take_snapshot(pageId=1)

// 4. Если заказчик прислал ТЗ или подтвердил сотрудничество:
// Выбираем диалог (click по uid диалога)
// Читаем последнее сообщение
```

---

## Рецепт 5: Сдача готовой работы в чат

```javascript
// 1. В открытом окне диалога с заказчиком делаем снимок
take_snapshot(pageId=1)

// 2. Находим input для загрузки файлов (скрепка) и загружаем архив с решением
upload_file(pageId=1, uid="<chat_file_input_uid>", filePath="C:/Users/Admin/.../solution.zip")

// 3. Заполняем поле сообщения с отчетом
fill(pageId=1, uid="<chat_textarea_uid>", value="Здравствуйте! Заказ полностью готов. Прикрепляю архив с кодом и инструкцией в README.md.")

// 4. Отправляем сообщение
click(pageId=1, uid="<chat_send_btn_uid>")
```

---

## Рецепт 6: Обработка всплывающих окон и CAPTCHA

```javascript
// 1. Если страница заблокирована проверкой Cloudflare / CAPTCHA:
take_screenshot(pageId=1, filePath="C:/Users/Admin/.../captcha_detected.png")

// 2. Агент информирует пользователя и ожидает решения проверки человеком:
// "Обнаружена CAPTCHA. Пожалуйста, пройдите проверку в браузере."

// 3. После подтверждения пользователя делаем повторный снимок:
take_snapshot(pageId=1)
```
