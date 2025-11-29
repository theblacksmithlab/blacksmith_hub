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

Все сервисы работают в Docker и слушают localhost:
- **uniframe_studio:** `127.0.0.1:8080`
- **the_viper_room:** `127.0.0.1:3001`
- **blacksmith_web:** `127.0.0.1:3000`
- **Боты:** не имеют HTTP интерфейса, работают через Telegram API
- **Агенты:** не имеют HTTP интерфейса, работают через Telegram User API

---

## Управление конфигурацией

### Применение изменений Nginx

```bash
# Проверить конфиг на ошибки
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
# Access log (последние 100 строк)
sudo tail -n 100 /var/log/nginx/api.blacksmith-lab.com.access.log

# Error log (в реальном времени)
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
# Разрешенные порты
sudo ufw allow 80/tcp   # HTTP
sudo ufw allow 443/tcp  # HTTPS
sudo ufw allow 22/tcp   # SSH

# Проверить статус
sudo ufw status
```

### Рекомендации
1. Регулярно обновлять сертификаты (certbot делает автоматически)
2. Мониторить логи на подозрительную активность
3. Использовать fail2ban для защиты от брутфорса
4. Регулярно обновлять Nginx: `sudo apt update && sudo apt upgrade nginx`

---

**Версия документа:** 1.1
**Дата создания:** 2025-11-23
**Последнее обновление:** 2025-11-27
**Изменения в 1.1:** Добавлен The Viper Room сервер (порт 3001), исправлена маршрутизация эндпоинтов
