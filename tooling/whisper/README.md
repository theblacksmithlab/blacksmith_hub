# Whisper Transcription Service

HTTP-сервис для транскрипции голосовых сообщений на базе whisper.cpp.

## 🎯 Назначение

Внутренний микросервис для транскрипции аудио в текст. Используется всеми Telegram ботами (probiot_bot, the_viper_room_bot, groot_bot) для обработки голосовых сообщений.

## 🚀 Быстрый старт

### 1. Добавить в .env

```bash
WHISPER_SERVICE_URL=http://whisper:9000
```

### 2. Запуск через docker-compose

```bash
# Из корня проекта
docker-compose up -d whisper

# Или через manager скрипт
./whisper_manager.sh start
./whisper_manager.sh status
```

### 3. Проверка работы

```bash
# Из другого контейнера в docker-compose
curl -X POST http://whisper:9000/transcribe \
  -F "audio=@test_voice.ogg"

# Ответ:
# {
#   "text": "Транскрибированный текст",
#   "duration_ms": 1234
# }
```

### 4. Просмотр логов

```bash
./whisper_manager.sh logs
# или
docker-compose logs -f whisper
```

---

## 📋 Команды управления

### Основные команды

| Команда | Описание |
|---------|----------|
| `./whisper_manager.sh start` | Запустить сервис |
| `./whisper_manager.sh stop` | Остановить сервис |
| `./whisper_manager.sh restart` | Перезапустить (без пересборки) |
| `./whisper_manager.sh rebuild [MODEL]` | Пересобрать и запустить |
| `./whisper_manager.sh logs` | Показать логи |
| `./whisper_manager.sh status` | Статус сервиса |

### Примеры

```bash
# Запуск сервиса
./whisper_manager.sh start

# Пересборка с другой моделью
./whisper_manager.sh rebuild small    # 466 MB (default)
./whisper_manager.sh rebuild medium   # 1.5 GB

# Просмотр статуса
./whisper_manager.sh status

# Логи
./whisper_manager.sh logs
```

---

## 🔧 Конфигурация

### config.yaml

```yaml
server:
  host: "0.0.0.0"
  port: 9000

cors:
  allowed_origins:
    - "*"
```

### Environment Variables

| Переменная | Значение по умолчанию | Описание |
|------------|----------------------|----------|
| `WHISPER_MODEL_PATH` | `/app/whisper.cpp/models/ggml-small.bin` | Путь к модели whisper |
| `CONFIG_PATH` | `/app/config.yaml` | Путь к конфигу (опционально) |

---

## 📊 Модели whisper

| Модель | Размер | Качество | Скорость | Использование |
|--------|--------|----------|----------|---------------|
| tiny | 75 MB | ⭐ | ⚡⚡⚡ | Только тесты |
| base | 142 MB | ⭐⭐ | ⚡⚡ | Быстро, низкое качество |
| **small** | 466 MB | **⭐⭐⭐** | **⚡** | **Баланс (продакшн, default)** |
| medium | 1.5 GB | ⭐⭐⭐⭐ | 🐢 | Лучшее качество |
| large | 2.9 GB | ⭐⭐⭐⭐⭐ | 🐌 | Максимальное качество |

**Рекомендация:** `small` — оптимальный баланс скорости и качества.

---

## 🔌 API

### POST /transcribe

Транскрибирует аудио-файл в текст.

**Request:**
```http
POST /transcribe HTTP/1.1
Content-Type: multipart/form-data

audio: <audio file (any format supported by ffmpeg)>
```

**Response:**
```json
{
  "text": "Распознанный текст",
  "duration_ms": 1234
}
```

**Поддерживаемые форматы:**
- OGG, MP3, WAV, M4A, FLAC, и другие (через ffmpeg)

---

## 🔗 Интеграция с ботами

### Использование в коде

```
use core::utils::common::transcribe_voice_message_http;


let transcription = transcribe_voice_message_http(&file_path).await?;
```

### Environment для ботов

Добавь в `.env`:

```bash
WHISPER_SERVICE_URL=http://whisper:9000
```

**Важно**: Используй `http://whisper:9000` (имя сервиса) для docker-compose, а не `127.0.0.1`!

---

## 🐳 Docker

### Интеграция через docker-compose

Whisper уже добавлен в `docker-compose.yml`:

```yaml
whisper:
  build:
    context: .
    dockerfile: docker/Dockerfile.whisper
    args:
      WHISPER_MODEL: medium
  container_name: whisper_service
  restart: unless-stopped
```

Управление через docker-compose или `./whisper_manager.sh`.

---

## 🔍 Troubleshooting

### Сервис не запускается

```bash
# Проверить логи
./whisper_manager.sh logs

# Проверить статус
./whisper_manager.sh status

# Пересобрать
./whisper_manager.sh rebuild medium
```

### Ошибка подключения из бота

**Проблема**: `error sending request for url (http://whisper:9000/transcribe)`

**Решение**: Проверьте:
1. Whisper запущен: `./whisper_manager.sh status`
2. Бот в той же docker-compose network
3. В `.env` указано: `WHISPER_SERVICE_URL=http://whisper:9000` (не 127.0.0.1!)
4. Перезапустите бота: `docker-compose restart probiot_bot`

### Ошибка "whisper-cli not found"

Модель не скачалась при сборке. Пересоберите:

```bash
./whisper_manager.sh rebuild medium
```

---

## 📈 Производительность

### Время транскрипции (модель small)

| Длительность аудио | Время обработки |
|-------------------|-----------------|
| 5 секунд | ~1-2 секунды |
| 30 секунд | ~5-10 секунд |
| 1 минута | ~10-20 секунд |

*На CPU Intel i7, без GPU ускорения*

### Требования

- **RAM:** 1-2 GB (для модели small)
- **CPU:** 2+ ядра рекомендуется
- **Диск:** 2 GB (образ + модель)

---

## 🔄 Миграция с локального whisper

### Старый подход (legacy):

```
use core::utils::common::transcribe_voice_message;

let transcription = transcribe_voice_message(&file_path).await?;
// ↑ Требует whisper-cli в контейнере, компилируется при каждом билде
```

### Новый подход (рекомендуется):

```
use core::utils::common::transcribe_voice_message_http;

let transcription = transcribe_voice_message_http(&file_path).await?;
// ↑ Использует HTTP-сервис, не требует whisper в контейнере бота
```

### Преимущества HTTP-подхода:

- ✅ **Быстрая сборка ботов** — не нужно компилировать whisper.cpp
- ✅ **Один whisper для всех** — переиспользование сервиса
- ✅ **Легко обновить модель** — пересобрать только whisper-сервис
- ✅ **Масштабируемость** — можно запустить несколько реплик

---

## 📝 Лицензия

Whisper.cpp: MIT License
Проект: Blacksmith Lab

---

**Версия:** 1.0
**Дата:** 2025-12-30
**Автор:** Blacksmith Lab Team
