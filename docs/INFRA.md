# Infrastructure Setup

Этот документ описывает инфраструктурную конфигурацию для продакшн-деплоя Blacksmith Lab.

---

## Nginx Configuration

### Обзор

Nginx выступает как reverse proxy для всех сервисов, обеспечивая:
- HTTPS терминацию с Let's Encrypt сертификатами
- Маршрутизацию запросов к соответствующим Docker контейнерам
- Статический контент для веб-интерфейсов

### Маршрутизация

| Path Pattern                                                     | Destination     | Port | SSL Internal |
|------------------------------------------------------------------|-----------------|------|--------------|
| `/api/uniframe/*`                                                | uniframe_studio | 8080 | HTTPS        |
| `/the_viper_room_user_request`, `/the_viper_room_avatar_request` | the_viper_room  | 3001 | HTTPS        |
| `/user_action`, `/get_user_avatar`, `/blacksmith_web_*`          | blacksmith_web  | 3000 | HTTPS        |
| `/` (default)                                                    | Static files    | -    | -            |

**Примечание:**
- Whisper сервис (порт 9000) доступен только внутри docker-compose network и не проксируется через nginx.
- `blacksmith_web` - мультитенантный сервер, обслуживающий Blacksmith Lab и W3A фронтенды. Разделение логики по HTTP origin.

### Домены

- **api.blacksmith-lab.com** - основной API домен
  - HTTP (80) → HTTPS redirect
  - HTTPS (443) → proxy к сервисам

### SSL/TLS

- **Сертификаты:** Let's Encrypt (автообновление)
- **Путь:** `/etc/letsencrypt/live/api.blacksmith-lab.com/`
- **Протоколы:** TLSv1.2, TLSv1.3

### Таймауты

**Uniframe Studio:**
- Connect: 30s
- Send: 30s
- Read: 30s

**The Viper Room:**
- Connect: 600s (10 минут)
- Send: 600s
- Read: 600s
- Причина: длительная генерация подкастов через LLM и Telegram API

**Blacksmith Web:**
- Connect: 600s (10 минут)
- Send: 600s
- Read: 600s
- Причина: длительные AI-запросы и обработка

### Логи

- **Access log:** `/var/log/nginx/api.blacksmith-lab.com.access.log`
- **Error log:** `/var/log/nginx/api.blacksmith-lab.com.error.log`

---

## Полный Nginx конфиг

**Файл:** `infra/nginx/default`

---

## Docker Architecture

### Multi-Stage Build Strategy

Проект использует multi-stage builds для оптимизации размера образов и переиспользования слоёв.

#### Базовые образы (base images)

**Назначение:** Общие слои для всех приложений одного типа

**bot_foundry_base:**
```dockerfile
# Строит bot_foundry бинарник
# Устанавливает: whisper.cpp, ffmpeg, OpenSSL, CA certificates
# Копирует entrypoint: infra/docker/entrypoint_bot_foundry.sh
# НЕ запускается сам - база для конкретных ботов
```

**the_forge_base:**
```dockerfile
# Строит the_forge бинарник
# Устанавливает: OpenSSL, CA certificates
# Копирует SSL сертификаты из ./certs/ в /etc/letsencrypt/
# Копирует entrypoint: infra/docker/entrypoint_the_forge.sh
# НЕ запускается сам - база для веб-сервисов
```

**agent_foundry_base:**
```dockerfile
# Строит agent_foundry бинарник
# Минимальная установка: CA certificates, OpenSSL
# Копирует entrypoint: infra/docker/entrypoint_agent_foundry.sh
# НЕ запускается сам - база для агентов
```

#### Конкретные сервисы (service images)

Наследуются от базовых образов и добавляют специфичную конфигурацию:
```dockerfile
# Пример: groot_bot
FROM bot_foundry_base

# Копирование специфичных ресурсов
COPY common_res/groot_bot/ /app/common_res/groot_bot/
COPY common_res/messages/ /app/common_res/messages/
COPY common_res/system_roles/ /app/common_res/system_roles/
COPY bot_foundry/config.yaml /app/config.yaml

# Определение какое приложение запустить
ENV APP_NAME="groot_bot"

# Базовый entrypoint проверяет APP_NAME и запускает нужный бинарник
```

#### Entrypoint Scripts

**infra/docker/entrypoint_bot_foundry.sh:**
- Проверяет наличие `APP_NAME` environment variable
- Запускает бинарник `bot_foundry` (который внутренне выбирает бот по APP_NAME)

**infra/docker/entrypoint_the_forge.sh:**
- Проверяет наличие `APP_NAME`
- Проверяет наличие `config.yaml`
- Запускает бинарник `the_forge`

**infra/docker/entrypoint_agent_foundry.sh:**
- Проверяет наличие `APP_NAME`
- Запускает бинарник `agent_foundry`

### Docker Compose Networks

**External services (через nginx):**
- `uniframe_studio:8080` → nginx → `/api/uniframe/`
- `the_viper_room:3001` → nginx → `/the_viper_room_*`
- `blacksmith_web:3000` → nginx → `/user_action`, `/blacksmith_web_*`

**Internal services (только docker-compose network):**
- `whisper:9000` - транскрипция (доступен только ботам)
- `qdrant:6333` - векторная БД для RAG

**Services без HTTP:**
- Боты - работают через Telegram Bot API
- Агенты - работают через Telegram User API

### Environment Variables

**Конфигурация:**
- `.env` — все переменные для локальной разработки и продакшна
- `docker-compose.yml` — импортирует переменные через `env_file: .env`
- `infra/docker/entrypoint_*.sh` — проверяет наличие обязательных переменных

**Критичная переменная для всех приложений:**
- `APP_NAME` — определяет какое приложение запустить

### Deployment Commands
```bash
# The Forge (веб-сервисы)
./the_forge_manager.sh build [blacksmith_web|uniframe_studio|the_viper_room]
./the_forge_manager.sh start <service>
./the_forge_manager.sh stop <service>
./the_forge_manager.sh logs <service>
./the_forge_manager.sh restart <service>

# Bot Foundry (боты)
./bot_foundry_manager.sh build [probiot_bot|groot_bot|the_viper_room_bot|stat_bot]
./bot_foundry_manager.sh start <bot>
./bot_foundry_manager.sh stop <bot>
./bot_foundry_manager.sh logs <bot>

# Agent Foundry (агенты)
./agent_foundry_manager.sh build [agent_davon]
./agent_foundry_manager.sh start <agent>
./agent_foundry_manager.sh stop <agent>
./agent_foundry_manager.sh logs <agent>
```

---

## Docker Troubleshooting

### Container не запускается
```bash
# Проверить логи
docker logs 

# Проверить APP_NAME
docker exec  env | grep APP_NAME

# Проверить entrypoint
docker exec  cat /entrypoint.sh
```

### Ошибка: "APP_NAME environment variable is not set"

**Причина:** Не установлена переменная `APP_NAME` в docker-compose.yaml или .env

**Решение:**
```yaml
# docker-compose.yaml
services:
  probiot_bot:
    environment:
      - APP_NAME=probiot_bot  # ← Добавь это
```

### Ошибка: "config.yaml not found"

**Причина:** Файл не скопирован в образ

**Решение:**
```bash
# Проверь COPY директиву в Dockerfile
docker exec  ls -la /app/config.yaml

# Пересобери с --no-cache
./bot_foundry_manager.sh build --no-cache probiot_bot
```

### База не скомпилировалась
```bash
# Очистить кэш и пересобрать
docker builder prune -f
./the_forge_manager.sh build 
```

---

## Whisper Service

### Назначение
Внутренний микросервис для транскрипции голосовых сообщений. Используется всеми Telegram ботами (probiot_bot, the_viper_room_bot, groot_bot).

### Интеграция в инфраструктуру

**Docker-compose конфигурация:**
```yaml
whisper:
  build:
    context: .
    dockerfile: infra/docker/Dockerfile.whisper
    args:
      WHISPER_MODEL: small
  container_name: whisper_service
  restart: unless-stopped
  deploy:
    resources:
      limits:
        cpus: '2'
        memory: 2G
```

**Networking:**
- **Внутренний доступ:** `http://whisper:9000` (только docker-compose network)
- **Внешний доступ:** НЕТ (не проксируется через nginx)
- **Используется:** ботами внутри docker-compose

### Управление

```bash
# Запуск/остановка
./whisper_manager.sh start
./whisper_manager.sh stop
./whisper_manager.sh restart

# Логи и статус
./whisper_manager.sh logs
./whisper_manager.sh status

# Пересборка с другой моделью
./whisper_manager.sh rebuild small   # default
./whisper_manager.sh rebuild medium  # лучше качество
```

### Дополнительная информация

**📖 Полная документация:** [tooling/whisper/README.md](../tooling/whisper/README.md)
- API спецификация
- Сравнение моделей
- Примеры интеграции
- Troubleshooting
- Производительность
