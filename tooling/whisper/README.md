# Whisper Transcription Service

HTTP-сервис для транскрипции голосовых сообщений на базе whisper.cpp.

> **📖 Документация по деплою и инфраструктуре:** [docs/INFRASTRUCTURE.md](../../docs/INFRASTRUCTURE.md#whisper-service)
> Этот README содержит API спецификацию, примеры использования и troubleshooting.

## 🎯 Назначение

Внутренний микросервис для транскрипции аудио в текст. Используется всеми Telegram ботами (probiot_bot, the_viper_room_bot, groot_bot) для обработки голосовых сообщений.

## 🚀 Быстрый старт

**Предварительные требования:**
- Whisper сервис развернут в docker-compose ([см. INFRASTRUCTURE.md](../../docs/INFRASTRUCTURE.md))
- `WHISPER_SERVICE_URL=http://whisper:9000` в `.env`

### Тестирование API

```bash
# Из другого контейнера в docker-compose
curl -X POST http://whisper:9000/transcribe \
  -F "audio=@test_voice.ogg"

# Ответ:
{
  "text": "Транскрибированный текст",
  "duration_ms": 1234
}
```

**Управление сервисом:** См. [docs/INFRASTRUCTURE.md](../../docs/INFRASTRUCTURE.md#whisper-service)

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


## 🔍 Troubleshooting

### Ошибка подключения из бота

**Проблема**: `error sending request for url (http://whisper:9000/transcribe)`

**Возможные причины:**
1. ❌ Whisper сервис не запущен
2. ❌ Бот не в той же docker-compose network
3. ❌ Неправильный URL в `.env`

**Решение:**
1. Проверь что сервис запущен ([см. INFRASTRUCTURE.md](../../docs/INFRASTRUCTURE.md#whisper-service))
2. Убедись что в `.env` указано: `WHISPER_SERVICE_URL=http://whisper:9000` (не 127.0.0.1!)
3. Перезапусти бота: `docker-compose restart <bot_name>`

### Медленная транскрипция

**Проблема**: Обработка голосового сообщения занимает слишком много времени.

**Решение:**
- Используй модель **small** вместо medium (в 2-3 раза быстрее)
- Увеличь CPU лимиты в docker-compose.yml
- Проверь загрузку CPU на хосте

### Ошибка транскрипции

**Проблема**: API возвращает ошибку 500

**Проверь:**
1. Логи whisper сервиса
2. Формат аудио файла (должен быть поддержан ffmpeg)
3. Размер файла (< 10 MB)

---

## 📈 Производительность

### Требования (модель small)

- **RAM:** 1-2 GB
- **CPU:** 2+ ядра рекомендуется
- **Диск:** 2 GB (образ + модель)

### Типичное время обработки

Модель **small** на CPU Intel i7:
- 5 сек аудио → ~1-2 сек
- 30 сек аудио → ~5-10 сек
- 1 мин аудио → ~10-20 сек

Модель **medium** медленнее в ~2-3 раза.

---

## 📚 Связанная документация

- **[docs/INFRASTRUCTURE.md](../../docs/INFRASTRUCTURE.md#whisper-service)** - Деплой, docker-compose, управление
- **[CLAUDE.md](../../CLAUDE.md)** - Общая архитектура проекта

---

**Версия:** 1.1
**Дата обновления:** 2025-12-30
**Изменения:** Реорганизация документации - разделение на инфраструктуру и использование
