# Whisper Transcription Service

HTTP-сервис для транскрипции голосовых сообщений на базе whisper.cpp.

## 🎯 Назначение

Внутренний микросервис для транскрипции аудио в текст. Используется всеми Telegram ботами (probiot_bot, the_viper_room_bot, groot_bot) для обработки голосовых сообщений.

## 🚀 Быстрый старт

### 1. Сборка и запуск

```bash
# Из корня проекта
./whisper_manager.sh build    # Собрать образ (модель medium по умолчанию)
./whisper_manager.sh start    # Запустить сервис
./whisper_manager.sh status   # Проверить статус
```

### 2. Проверка работы

```bash
# Тест endpoint
curl -X POST http://127.0.0.1:9000/transcribe \
  -F "audio=@test_voice.ogg"

# Ответ:
# {
#   "text": "Транскрибированный текст",
#   "duration_ms": 1234
# }
```

### 3. Просмотр логов

```bash
./whisper_manager.sh logs
```

---

## 📋 Команды управления

### Основные команды

| Команда | Описание |
|---------|----------|
| `./whisper_manager.sh build [MODEL]` | Собрать Docker-образ |
| `./whisper_manager.sh start` | Запустить сервис |
| `./whisper_manager.sh stop` | Остановить сервис |
| `./whisper_manager.sh restart` | Перезапустить (без пересборки) |
| `./whisper_manager.sh rebuild [MODEL]` | Пересобрать и запустить |
| `./whisper_manager.sh logs` | Показать логи |
| `./whisper_manager.sh status` | Статус сервиса |
| `./whisper_manager.sh clean` | Удалить контейнер и образ |

### Примеры

```bash
# Сборка с разными моделями
./whisper_manager.sh build medium    # 1.5 GB, лучшее качество (default)
./whisper_manager.sh build small     # 466 MB, баланс
./whisper_manager.sh build base      # 142 MB, быстрее

# Пересборка с другой моделью
./whisper_manager.sh rebuild small

# Просмотр статуса
./whisper_manager.sh status
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
| `WHISPER_MODEL_PATH` | `/app/whisper.cpp/models/ggml-medium.bin` | Путь к модели whisper |
| `CONFIG_PATH` | `/app/config.yaml` | Путь к конфигу (опционально) |

---

## 📊 Модели whisper

| Модель | Размер | Качество | Скорость | Использование |
|--------|--------|----------|----------|---------------|
| tiny | 75 MB | ⭐ | ⚡⚡⚡ | Только тесты |
| base | 142 MB | ⭐⭐ | ⚡⚡ | Быстро, низкое качество |
| small | 466 MB | ⭐⭐⭐ | ⚡ | Баланс |
| **medium** | 1.5 GB | **⭐⭐⭐⭐** | 🐢 | **Рекомендуется (продакшн)** |
| large | 2.9 GB | ⭐⭐⭐⭐⭐ | 🐌 | Максимальное качество |

**Рекомендация:** `medium` — оптимальный выбор для русской речи.

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

Добавь в `.env` или Docker environment:

```bash
WHISPER_SERVICE_URL=http://127.0.0.1:9000
```

Если не указано, используется `http://127.0.0.1:9000` по умолчанию.

---

## 🐳 Docker

### Ручная сборка

```bash
# С моделью medium (default)
docker build -f docker/Dockerfile.whisper -t whisper:latest .

# С другой моделью
docker build -f docker/Dockerfile.whisper \
  --build-arg WHISPER_MODEL=small \
  -t whisper:latest .
```

### Ручный запуск

```bash
docker run -d \
  --name whisper_service \
  -p 127.0.0.1:9000:9000 \
  --restart unless-stopped \
  whisper:latest
```

---

## 🔍 Troubleshooting

### Сервис не запускается

```bash
# Проверить логи
./whisper_manager.sh logs

# Проверить статус
./whisper_manager.sh status

# Пересоздать контейнер
./whisper_manager.sh clean
./whisper_manager.sh build
./whisper_manager.sh start
```

### Ошибка "whisper-cli not found"

Модель не скачалась при сборке. Пересоберите образ:

```bash
./whisper_manager.sh rebuild medium
```

### Ошибка подключения из бота

Проверьте:
1. Whisper сервис запущен: `./whisper_manager.sh status`
2. Порт доступен: `curl http://127.0.0.1:9000/transcribe -X POST`
3. `WHISPER_SERVICE_URL` правильно указан в env бота

---

## 📈 Производительность

### Время транскрипции (модель medium)

| Длительность аудио | Время обработки |
|-------------------|-----------------|
| 5 секунд | ~2-3 секунды |
| 30 секунд | ~10-15 секунд |
| 1 минута | ~20-30 секунд |

*На CPU Intel i7, без GPU ускорения*

### Требования

- **RAM:** 2 GB минимум (для модели medium)
- **CPU:** 2+ ядра рекомендуется
- **Диск:** 3 GB (образ + модель)

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
