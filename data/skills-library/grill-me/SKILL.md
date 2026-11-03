---
name: grill-me
description: "Interactive interview and requirement alignment protocol. Use whenever requirements are ambiguous, multiple technical/architectural options exist, or agent-preset-conductor detects a disputable domain. Systematically interviews the user one question at a time using ask_question to resolve design branches."
---

# Grill-Me: Interactive Design & Requirements Interview Protocol

Интерактивный протокол согласования требований и разрешения архитектурных развилок.

## Когда активируется
1. **По сигналу `agent-preset-conductor`**: когда задача затрагивает несколько доменов (Web + QA, Backend + DevOps) или выбор пресета является спорным.
2. **Неоднозначные требования**: когда существует несколько альтернативных архитектурных путей с компромиссами.
3. **По запросу пользователя**: при явном упоминании `/grill-me` или просьбе составить план.

## Строгие правила интервью
1. **Строго один вопрос за раз**: никогда не засыпать пользователя пачкой вопросов. Задавать ровно один вопрос за ход.
2. **Использовать инструмент `ask_question`**: всегда формулировать варианты с помощью инструмента `ask_question`.
3. **Рекомендованный вариант первым**: ставить лучший вариант первым с префиксом `(Recommended)`.
4. **Спускаться по дереву решений от корня к листьям**: сначала решать фундаментальные развилки (стек, стратегия), затем детали реализации.
5. **Сначала исследовать кодовую базу**: если ответ можно найти в коде или конфигах, исследовать код самостоятельно вместо лишнего вопроса.
