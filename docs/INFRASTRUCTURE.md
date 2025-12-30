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

| Path Pattern | Destination | Port | SSL Internal |
|-------------|-------------|------|--------------|
| `/api/uniframe/*` | uniframe_studio | 8080 | HTTPS |
| `/the_viper_room_user_request`, `/the_viper_room_avatar_request` | the_viper_room | 3001 | HTTPS |
| `/user_action`, `/get_user_avatar`, `/blacksmith_web_*` | blacksmith_web | 3000 | HTTPS |
| `/` (default) | Static files | - | - |

**Примечание:** Whisper сервис (порт 9000) доступен только внутри docker-compose network и не проксируется через nginx.

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

**Файл:** `/etc/nginx/sites-available/default`

```nginx
# HTTP redirect to HTTPS
server {
    listen 80;
    server_name api.blacksmith-lab.com;
    return 301 https://$host$request_uri;
}

# HTTPS
server {
    listen 443 ssl;
    server_name api.blacksmith-lab.com;

    # SSL certs
    ssl_certificate /etc/letsencrypt/live/api.blacksmith-lab.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/api.blacksmith-lab.com/privkey.pem;
    ssl_protocols TLSv1.2 TLSv1.3;

    # Logs
    access_log /var/log/nginx/api.blacksmith-lab.com.access.log;
    error_log /var/log/nginx/api.blacksmith-lab.com.error.log;
    
    # Uniframe Studio API
    location /api/uniframe/ {
        proxy_pass http://127.0.0.1:8080;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_connect_timeout 30s;
        proxy_send_timeout 30s;
        proxy_read_timeout 30s;
    }

    # The Viper Room API (HTTP - TLS terminated at nginx)
    location ~ ^/(the_viper_room_user_request|the_viper_room_avatar_request) {
        proxy_pass http://127.0.0.1:3001;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_pass_request_headers on;
        proxy_connect_timeout 600s;
        proxy_send_timeout 600s;
        proxy_read_timeout 600s;
        send_timeout 600s;
    }

    # API endpoints (Blacksmith Web)
    location ~ ^/(user_action|get_user_avatar|blacksmith_web_user_request|blacksmith_web_chat_fetch|blacksmith_web_tts_request) {
        proxy_pass http://127.0.0.1:3000;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_pass_request_headers on;
        proxy_connect_timeout 600s;
        proxy_send_timeout 600s;
        proxy_read_timeout 600s;
        send_timeout 600s;
    }

    # Static files
    location / {
        root /var/www/html;
        index index.html;
    }
}
```

---

## Docker Networks

### Сервисы с внешним доступом (через nginx):
- **uniframe_studio:** `127.0.0.1:8080` → nginx → `/api/uniframe/`
- **the_viper_room:** `127.0.0.1:3001` → nginx → `/the_viper_room_*`
- **blacksmith_web:** `127.0.0.1:3000` → nginx → `/user_action`, `/blacksmith_web_*`

### Внутренние сервисы (только docker-compose network):
- **whisper:** `whisper:9000` - доступен только для ботов внутри docker-compose
- **qdrant:** `qdrant:6333` - векторная БД для RAG-системы

### Сервисы без HTTP интерфейса:
- **Боты** (probiot_bot, groot_bot, the_viper_room_bot) - работают через Telegram Bot API
- **Агенты** (agent_davon) - работают через Telegram User API

---

## Управление конфигурацией

### Применение изменений Nginx

```bash
sudo nginx -t

# Перезагрузить конфигурацию (без downtime)
sudo systemctl reload nginx

# Полный перезапуск
sudo systemctl restart nginx
```

### Обновление SSL сертификатов

```bash
# Обновить сертификаты (автоматически через certbot)
sudo certbot renew

# Проверить дату истечения
sudo certbot certificates
```

### Просмотр логов

```bash
sudo tail -n 100 /var/log/nginx/api.blacksmith-lab.com.access.log

sudo tail -f /var/log/nginx/api.blacksmith-lab.com.error.log
```

---

## Troubleshooting

### Nginx не запускается
```bash
# Проверить синтаксис конфига
sudo nginx -t

# Проверить статус
sudo systemctl status nginx

# Посмотреть детальные ошибки
sudo journalctl -u nginx -n 50
```

### 502 Bad Gateway
- Проверить работают ли Docker контейнеры: `docker ps`
- Проверить логи сервиса: `docker logs blacksmith_web`
- Проверить что порты открыты: `netstat -tlnp | grep -E '3000|8080'`

### SSL ошибки
- Проверить валидность сертификатов: `sudo certbot certificates`
- Проверить права доступа: `ls -la /etc/letsencrypt/live/api.blacksmith-lab.com/`

---

## Безопасность

### Firewall (ufw)
```bash
sudo ufw allow 80/tcp   # HTTP
sudo ufw allow 443/tcp  # HTTPS
sudo ufw allow 22/tcp   # SSH

sudo ufw status
```

---

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
    dockerfile: docker/Dockerfile.whisper
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

**Environment Variables для ботов:**

Добавить в `.env`:
```bash
WHISPER_SERVICE_URL=http://whisper:9000
```

**⚠️ Важно:** Используй `http://whisper:9000` (имя сервиса в docker-compose), а не `127.0.0.1`!

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

---

**Версия документа:** 1.4
**Дата создания:** 2025-11-23
**Последнее обновление:** 2025-12-30

**История изменений:**
- **1.4 (2025-12-30):** Реорганизация Whisper документации - инфраструктура в INFRASTRUCTURE.md, API в tooling/whisper/README.md
- **1.3 (2025-12-30):** Whisper Service - переход на docker-compose, модель small
- **1.2 (2025-12-30):** Добавлен Whisper Service
- **1.1:** Добавлен Uniframe Studio
- **1.0 (2025-11-23):** Первая версия
