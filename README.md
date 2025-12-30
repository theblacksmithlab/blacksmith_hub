# 🔧 Blacksmith Lab

> Продакшн-готовая платформа на Rust для AI-сервисов, Telegram ботов и веб-приложений

**Blacksmith Lab** — это монорепозиторий, объединяющий экосистему сервисов на базе Telegram Bot API, веб-API и AI-инструментов. Проект построен на архитектуре Rust workspace, где общая библиотека `core` переиспользуется множеством специализированных приложений.

---

## 🎯 Что внутри?

Blacksmith Lab включает в себя:

### 🌐 Веб-сервисы (The Forge)
- **Blacksmith Web** - AI-агент поддержки для сайта Blacksmith Lab
- **W3A Web** - AI-ассистент для онлайн-школы Web3 Academy
- **The Viper Room** - Генератор аудио-подкастов через LLM
- **Uniframe Studio** - Адаптивный дубляж видео с интеграцией Python ML-моделей

### 🤖 Telegram боты (Bot Foundry)
- **Probiot Bot** - Универсальный бот с RAG-системой для ответов на вопросы
- **The Viper Room Bot** - Telegram Mini-App для генерации подкастов
- **Groot Bot** - Модерация чатов, антиспам и управление подписками

### 🕵️ Telegram агенты (Agent Foundry)
- **Agent Davon** - Мониторинг публичных чатов на спам/скам (user-mode агент)

### 🛠️ Вспомогательные сервисы
- **Whisper Service** - HTTP-микросервис транскрибации голосовых сообщений

---

## 🏗️ Архитектура монорепо

```
┌─────────────────────────────────────────────────────────────┐
│                         CORE Library                        │
│  (общий код: AI, RAG, database, Telegram, utils, models)   │
└─────────────────────────────────────────────────────────────┘
                              ▲
                              │
        ┌─────────────────────┼─────────────────────┐
        │                     │                     │
        ▼                     ▼                     ▼
┌──────────────┐    ┌──────────────┐    ┌──────────────┐
│  THE FORGE   │    │ BOT FOUNDRY  │    │AGENT FOUNDRY │
│              │    │              │    │              │
│ Web Servers  │    │ Telegram     │    │ User-mode    │
│ (Axum)       │    │ Bots         │    │ Telegram     │
│              │    │ (Teloxide)   │    │ Agents       │
│              │    │              │    │ (Grammers)   │
└──────────────┘    └──────────────┘    └──────────────┘
        │                   │                     │
        ▼                   ▼                     ▼
┌──────────────┐    ┌──────────────┐    ┌──────────────┐
│• Blacksmith  │    │• Probiot Bot │    │• Agent Davon │
│  Web         │    │• Groot Bot   │    │              │
│• W3A Web     │    │• The Viper   │    │              │
│• The Viper   │    │  Room Bot    │    │              │
│  Room        │    │              │    │              │
│• Uniframe    │    │              │    │              │
│  Studio      │    │              │    │              │
└──────────────┘    └──────────────┘    └──────────────┘
```

### Как это работает?

**Один бинарник — много приложений:**
- `the_forge` компилируется один раз и запускает разные веб-сервисы в зависимости от переменной окружения `APP_NAME`
- `bot_foundry` компилируется один раз и запускает разные боты через `APP_NAME`
- `agent_foundry` - отдельные бинарники для user-mode агентов

**Общая библиотека:**
- Все приложения используют `core` для AI, баз данных, обработки сообщений и интеграций
- Это позволяет избежать дублирования кода и поддерживать единый стандарт

---

## 📦 Компоненты

### `core/` — Общая библиотека

Переиспользуемая библиотека для всех приложений в монорепо:

- **`ai/`** - Интеграция с LLM (OpenAI API)
- **`rag_system/`** - Полноценная RAG-система с векторным поиском (Qdrant)
- **`local_db/`** - Слой работы с SQLite
- **`telegram_client/`** - User-mode Telegram клиент (grammers)
- **`message_processing_flow/`** - Пайплайны обработки сообщений
- **`models/`** - Доменные модели для всех сервисов
- **`state/`** - Управление состоянием приложений
- **`utils/`** - Вспомогательные функции и утилиты

### `the_forge/` — Веб-серверы

Axum-based веб-серверы с TLS, CORS и reverse proxy через Nginx:

- **Blacksmith Web** (порт 3000) - Веб-интерфейс с AI-агентом поддержки
- **The Viper Room** (порт 3001) - Генерация подкастов через AI
- **Uniframe Studio** (порт 8080) - API для дубляжа видео

Выбор сервера: через переменную окружения `APP_NAME`

### `bot_foundry/` — Telegram боты

Telegram боты на базе Teloxide:

- **Probiot Bot** - RAG-система для ответов на вопросы пользователей
- **The Viper Room Bot** - Интерфейс к подкаст-платформе через Telegram
- **Groot Bot** - Модерация, антиспам, управление подписками в чатах

Выбор бота: через переменную окружения `APP_NAME`

### `agent_foundry/` — User-mode агенты

Telegram агенты, работающие от имени пользовательских аккаунтов:

- **Agent Davon** - Мониторит публичные чаты, детектит спам/скам через LLM-анализ, генерирует CSV-отчёты

### `tooling/whisper/` — Whisper сервис

HTTP-микросервис для транскрибации голосовых сообщений:

- Построен на whisper.cpp
- Используется всеми Telegram ботами
- Избавляет от необходимости компилировать whisper.cpp в каждом Docker-образе
- Работает внутри docker-compose network

---

## 📂 Структура директорий

```
blacksmith_lab/
├── core/                    # Общая библиотека
├── the_forge/              # Веб-серверы
├── bot_foundry/            # Telegram боты
├── agent_foundry/          # User-mode агенты
├── tooling/
│   └── whisper/            # Whisper HTTP-сервис
├── docker/                 # Dockerfiles и скрипты
├── docs/                   # Документация
│   ├── INFRASTRUCTURE.md   # Nginx, SSL, deployment
│   └── ...
├── common_res/             # Общие ресурсы (system_roles, messages, БД)
├── CLAUDE.md               # Техническая документация для разработчиков
└── README.md               # 👈 Вы здесь
```

---

## 🚀 Deployment

Проект использует **multi-stage Docker builds** с базовыми образами:

- `bot_foundry_base` → конкретные боты (groot_bot, probiot_bot, etc.)
- `the_forge_base` → веб-сервисы (blacksmith_web, uniframe_studio, etc.)
- `agent_foundry_base` → агенты (agent_davon, etc.)

Управление сервисами через bash-скрипты:
```bash
./the_forge_manager.sh --help
./bot_foundry_manager.sh --help
./agent_foundry_manager.sh --help
./whisper_manager.sh --help
```

---

## 📚 Документация

- **`docs/INFRASTRUCTURE.md`** - Инфраструктура и деплой:
  - Nginx конфигурация
  - SSL/TLS настройка
  - Docker networks
  - Логи и troubleshooting

- **`tooling/whisper/README.md`** - Whisper сервис:
  - API спецификация
  - Интеграция с ботами
  - Модели и производительность

---

## 🤝 Contributing

Перед началом работы:

1. **Прочитай `CLAUDE.md`** - там описаны все паттерны, подходы и правила
2. **Изучи существующий код** - следуй тем же паттернам и стилю
3. **Уточняй перед рефакторингом** - изменения в `core` влияют на все приложения
4. **Не коммить секреты** - используй `.env` для чувствительных данных

### Основные правила:
- ✅ Следуй существующим паттернам
- ✅ Используй `anyhow` для error handling
- ✅ Используй `tracing` для логирования
- ✅ Тестируй локально перед PR
- ❌ Не меняй workspace структуру без обсуждения
- ❌ Не коммить `.env` и секреты

---

## 🛡️ Лицензия

Proprietary - Blacksmith Lab Team

---

**Версия:** 1.0
**Дата создания:** 2025-12-30
**Maintained by:** Blacksmith Lab Team
