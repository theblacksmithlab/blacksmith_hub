# Whisper Transcription Service

HTTP-сервис для транскрипции голосовых сообщений на базе whisper.cpp.

**Используется:** Всеми Telegram ботами (probiot_bot, the_viper_room_bot, groot_bot)
**Деплой и управление:** [docs/INFRA.md](../../docs/INFRA.md#whisper-service)

---

## 🔌 API

### POST /transcribe

**Request:**
```http
POST /transcribe HTTP/1.1
Content-Type: multipart/form-data

audio: <audio file (OGG, MP3, WAV, M4A, FLAC)>
```

**Response:**
```json
{
  "text": "Распознанный текст",
  "duration_ms": 1234
}
```

**Пример:**
```bash
curl -X POST http://whisper:9000/transcribe -F "audio=@voice.ogg"
```

---

## 🔗 Интеграция с ботами

**Код:**
```
use core::utils::common::transcribe_voice_message_http;

let transcription = transcribe_voice_message_http(&file_path).await?;
```

**Environment в `.env`:**
```bash
WHISPER_SERVICE_URL=http://whisper:9000  # Docker-compose network
```

---

## 🔧 Конфигурация

**config.yaml:**
```yaml
server:
  host: "0.0.0.0"
  port: 9000

cors:
  allowed_origins:
    - "*"
```

**Environment (внутри Docker):**
| Переменная           | Значение по умолчанию                    | Описание               |
|----------------------|------------------------------------------|------------------------|
| `WHISPER_MODEL_PATH` | `/app/whisper.cpp/models/ggml-small.bin` | Путь к модели whisper  |
| `CONFIG_PATH`        | `/app/config.yaml`                       | Путь к конфигу сервиса |

**Смена модели:** `./whisper_manager.sh rebuild <model>`

---

## 📊 Модели whisper

| Модель  | Размер | Качество         | Скорость     | Использование              |
|---------|--------|------------------|--------------|----------------------------|
| tiny    | 75 MB  | ⭐                | ⚡⚡⚡          | Только тесты               |
| base    | 142 MB | ⭐⭐               | ⚡⚡           | Быстро, низкое качество    |
| small   | 466 MB | ⭐⭐⭐              | ⚡            | Баланс (продакшн, default) |
| medium  | 1.5 GB | ⭐⭐⭐⭐             | 🐢           | Лучшее качество            |
| large   | 2.9 GB | ⭐⭐⭐⭐⭐            | 🐌           | Максимальное качество      |

**Рекомендация:** `small` — оптимальный баланс.

---

## 🔍 Troubleshooting

### Ошибка подключения из бота
```
error sending request for url (http://whisper:9000/transcribe)
```

**Причины:**
- ❌ Сервис не запущен → проверь `docker ps | grep whisper`
- ❌ Неправильный URL в `.env` → должно быть `http://whisper:9000` (не 127.0.0.1)
- ❌ Бот не в docker-compose network → проверь networks в docker-compose.yml

### Медленная транскрипция
- Используй модель `small` вместо `medium` (в 2-3 раза быстрее)
- Увеличь CPU limits в docker-compose.yml
- Проверь `docker stats whisper_service`

### API возвращает 500
**Проверь:**
1. Логи: `docker logs whisper_service`
2. Формат аудио (должен быть поддержан ffmpeg)
3. Размер файла (< 10 MB рекомендуется)

---

## 📈 Производительность (reference)

**Требования (модель small):**
- RAM: 1-2 GB
- CPU: 2+ ядра рекомендуется
- Диск: 2 GB (образ + модель)

**Время обработки (small на Intel i7):**
- 5 сек аудио → ~1-2 сек
- 30 сек аудио → ~5-10 сек
- 1 мин аудио → ~10-20 сек

Модель `medium` медленнее в ~2-3 раза.

---

## 📚 См. также

- [INFRA.md](../../docs/INFRA.md#whisper-service) - Деплой, docker-compose, управление
- [CLAUDE.md](../../CLAUDE.md) - Общая архитектура проекта
