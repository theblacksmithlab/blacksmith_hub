#!/bin/bash

# =============================================================================
# Docker Cleanup Script - Безопасная очистка Docker мусора
# =============================================================================

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

print_message() {
    local color=$1
    local message=$2
    echo -e "${color}${message}${NC}"
}

print_header() {
    echo ""
    print_message "${BLUE}" "=============================================="
    print_message "${BLUE}" "$1"
    print_message "${BLUE}" "=============================================="
}

show_disk_usage() {
    print_header "📊 Текущее использование диска Docker"
    docker system df
    echo ""
    docker system df -v | grep -E "REPOSITORY|Images space usage" | head -10
}

# =============================================================================
# УРОВЕНЬ 1: БЕЗОПАСНАЯ ОЧИСТКА (можно запускать регулярно)
# =============================================================================
safe_cleanup() {
    print_header "🧹 БЕЗОПАСНАЯ ОЧИСТКА (Уровень 1)"

    print_message "${CYAN}" "Что будет удалено:"
    echo "  ✓ Dangling images (<none>:<none>)"
    echo "  ✓ Stopped containers"
    echo "  ✓ Unused networks"
    echo ""
    print_message "${GREEN}" "Что НЕ будет тронуто:"
    echo "  ✓ Running containers"
    echo "  ✓ Базовые образы (bot_foundry_base, the_forge_base, etc.)"
    echo "  ✓ Активные service images"
    echo "  ✓ BuildKit cache"
    echo "  ✓ Cargo cache volumes"
    echo ""

    read -p "Продолжить? (y/n): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        print_message "${YELLOW}" "Отменено"
        return
    fi

    print_message "${GREEN}" "1. Удаление dangling images..."
    DANGLING=$(docker images -f "dangling=true" -q | wc -l)
    if [ "$DANGLING" -gt 0 ]; then
        docker image prune -f
        print_message "${GREEN}" "   ✓ Удалено $DANGLING dangling images"
    else
        print_message "${YELLOW}" "   ⊘ Нет dangling images"
    fi

    print_message "${GREEN}" "2. Удаление stopped containers..."
    STOPPED=$(docker ps -a -f "status=exited" -f "status=created" -q | wc -l)
    if [ "$STOPPED" -gt 0 ]; then
        docker container prune -f
        print_message "${GREEN}" "   ✓ Удалено $STOPPED stopped containers"
    else
        print_message "${YELLOW}" "   ⊘ Нет stopped containers"
    fi

    print_message "${GREEN}" "3. Удаление unused networks..."
    docker network prune -f
    print_message "${GREEN}" "   ✓ Очистка завершена"

    echo ""
    print_message "${GREEN}" "✅ Безопасная очистка завершена!"
}

# =============================================================================
# УРОВЕНЬ 2: УМЕРЕННАЯ ОЧИСТКА (раз в месяц)
# =============================================================================
moderate_cleanup() {
    print_header "🔧 УМЕРЕННАЯ ОЧИСТКА (Уровень 2)"

    print_message "${CYAN}" "Что будет удалено:"
    echo "  ✓ Всё из Уровня 1"
    echo "  ✓ BuildKit cache старше 7 дней"
    echo "  ✓ Неиспользуемые volumes (с подтверждением)"
    echo ""
    print_message "${YELLOW}" "Что НЕ будет тронуто:"
    echo "  ✓ Running containers"
    echo "  ✓ Базовые образы"
    echo "  ✓ Активные service images"
    echo "  ✓ Свежий BuildKit cache (< 7 дней)"
    echo ""

    print_message "${YELLOW}" "⚠️  Эта очистка удалит старый build cache!"
    print_message "${YELLOW}" "⚠️  Следующий rebuild может быть медленнее"
    echo ""

    read -p "Продолжить? (y/n): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        print_message "${YELLOW}" "Отменено"
        return
    fi

    # Сначала безопасная очистка
    safe_cleanup

    echo ""
    print_message "${GREEN}" "4. Удаление старого BuildKit cache (> 7 дней)..."
    docker buildx prune --filter "until=168h" --force
    print_message "${GREEN}" "   ✓ Старый cache удалён"

    print_message "${YELLOW}" "5. Проверка unused volumes..."
    VOLUMES=$(docker volume ls -qf "dangling=true" | wc -l)
    if [ "$VOLUMES" -gt 0 ]; then
        echo "   Найдено $VOLUMES неиспользуемых volumes:"
        docker volume ls -f "dangling=true"
        echo ""
        read -p "   Удалить их? (y/n): " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            docker volume prune -f
            print_message "${GREEN}" "   ✓ Volumes удалены"
        else
            print_message "${YELLOW}" "   ⊘ Volumes оставлены"
        fi
    else
        print_message "${YELLOW}" "   ⊘ Нет unused volumes"
    fi

    echo ""
    print_message "${GREEN}" "✅ Умеренная очистка завершена!"
}

# =============================================================================
# УРОВЕНЬ 3: АГРЕССИВНАЯ ОЧИСТКА (только при критической нехватке места!)
# =============================================================================
aggressive_cleanup() {
    print_header "⚠️  АГРЕССИВНАЯ ОЧИСТКА (Уровень 3)"

    print_message "${RED}" "⚠️  ВНИМАНИЕ! Эта очистка удалит:"
    echo "  ✓ ВСЁ из Уровня 1 и 2"
    echo "  ✓ ВСЕ неиспользуемые images (включая старые базовые!)"
    echo "  ✓ ВЕСЬ BuildKit cache"
    echo "  ✓ ВСЕ Cargo cache volumes"
    echo ""
    print_message "${RED}" "После этого:"
    echo "  ✗ Следующий rebuild будет ОЧЕНЬ медленным (как первый раз)"
    echo "  ✗ Придётся заново качать все Rust dependencies"
    echo "  ✗ Придётся заново компилировать whisper.cpp"
    echo ""
    print_message "${YELLOW}" "💡 Используй это ТОЛЬКО если:"
    echo "  - Критическая нехватка места на диске"
    echo "  - Готов ждать долгий rebuild"
    echo ""

    read -p "Ты УВЕРЕН? Это удалит весь кеш! (yes/no): " -r
    echo
    if [[ ! $REPLY == "yes" ]]; then
        print_message "${YELLOW}" "Отменено (правильное решение!)"
        return
    fi

    print_message "${RED}" "Последнее предупреждение!"
    read -p "Набери 'DELETE ALL CACHE' для подтверждения: " -r
    echo
    if [[ ! $REPLY == "DELETE ALL CACHE" ]]; then
        print_message "${YELLOW}" "Отменено"
        return
    fi

    # Сначала умеренная очистка
    moderate_cleanup

    echo ""
    print_message "${RED}" "6. Удаление ВСЕХ неиспользуемых images..."
    docker image prune -a -f
    print_message "${GREEN}" "   ✓ Images удалены"

    print_message "${RED}" "7. Удаление ВСЕГО BuildKit cache..."
    docker buildx prune --all --force
    print_message "${GREEN}" "   ✓ BuildKit cache удалён"

    print_message "${RED}" "8. Удаление Cargo cache volumes..."
    docker volume ls -q | grep -E "cache.*cargo" | xargs -r docker volume rm
    print_message "${GREEN}" "   ✓ Cargo cache удалён"

    echo ""
    print_message "${GREEN}" "✅ Агрессивная очистка завершена!"
    print_message "${YELLOW}" "⚠️  Следующий build будет медленным - это нормально"
}

# =============================================================================
# DRY RUN режим (показать что будет удалено, но не удалять)
# =============================================================================
dry_run() {
    print_header "🔍 DRY RUN - Показать что можно удалить"

    echo ""
    print_message "${CYAN}" "📦 Dangling images:"
    DANGLING=$(docker images -f "dangling=true" -q | wc -l)
    if [ "$DANGLING" -gt 0 ]; then
        docker images -f "dangling=true"
        echo "Итого: $DANGLING images"
    else
        print_message "${GREEN}" "  ✓ Нет dangling images"
    fi

    echo ""
    print_message "${CYAN}" "🛑 Stopped containers:"
    STOPPED=$(docker ps -a -f "status=exited" -f "status=created" -q | wc -l)
    if [ "$STOPPED" -gt 0 ]; then
        docker ps -a -f "status=exited" -f "status=created" --format "table {{.Names}}\t{{.Status}}\t{{.Size}}"
        echo "Итого: $STOPPED containers"
    else
        print_message "${GREEN}" "  ✓ Нет stopped containers"
    fi

    echo ""
    print_message "${CYAN}" "🌐 Unused networks:"
    docker network ls --filter "dangling=true"

    echo ""
    print_message "${CYAN}" "💾 Unused volumes:"
    VOLUMES=$(docker volume ls -qf "dangling=true" | wc -l)
    if [ "$VOLUMES" -gt 0 ]; then
        docker volume ls -f "dangling=true"
        echo "Итого: $VOLUMES volumes"
    else
        print_message "${GREEN}" "  ✓ Нет unused volumes"
    fi

    echo ""
    print_message "${CYAN}" "🔨 BuildKit cache:"
    docker buildx du
}

# =============================================================================
# Показать help
# =============================================================================
show_help() {
    print_message "${BLUE}" "=== Docker Cleanup Script ==="
    echo ""
    print_message "${YELLOW}" "Использование:"
    echo "  ./docker_cleanup.sh [ОПЦИЯ]"
    echo ""
    print_message "${YELLOW}" "Опции:"
    echo "  safe | 1       - Безопасная очистка (рекомендуется регулярно)"
    echo "  moderate | 2   - Умеренная очистка (раз в месяц)"
    echo "  aggressive | 3 - Агрессивная очистка (только при критической нехватке места!)"
    echo "  dry-run        - Показать что можно удалить (без удаления)"
    echo "  status         - Показать текущее использование диска"
    echo "  help           - Показать эту справку"
    echo ""
    print_message "${YELLOW}" "Интерактивный режим (без параметров):"
    echo "  ./docker_cleanup.sh"
    echo ""
    print_message "${CYAN}" "📖 Рекомендации:"
    echo "  • Безопасная очистка    - можно запускать каждую неделю"
    echo "  • Умеренная очистка     - раз в месяц или когда мало места"
    echo "  • Агрессивная очистка   - только в крайнем случае!"
}

# =============================================================================
# Интерактивное меню
# =============================================================================
interactive_menu() {
    while true; do
        print_header "🐳 Docker Cleanup - Выбери уровень очистки"
        echo ""
        echo "1) Безопасная очистка (рекомендуется)"
        echo "   └─ Dangling images + stopped containers + unused networks"
        echo ""
        echo "2) Умеренная очистка (раз в месяц)"
        echo "   └─ Уровень 1 + старый BuildKit cache (>7 дней) + unused volumes"
        echo ""
        echo "3) Агрессивная очистка (ОПАСНО! Только при критической нехватке места)"
        echo "   └─ Уровень 2 + ВСЕ неиспользуемые images + ВЕСЬ cache"
        echo ""
        echo "4) Dry Run (показать что можно удалить)"
        echo "5) Показать статистику использования диска"
        echo "6) Выход"
        echo ""
        read -p "Выбери опцию (1-6): " choice

        case $choice in
            1)
                show_disk_usage
                safe_cleanup
                echo ""
                show_disk_usage
                ;;
            2)
                show_disk_usage
                moderate_cleanup
                echo ""
                show_disk_usage
                ;;
            3)
                show_disk_usage
                aggressive_cleanup
                echo ""
                show_disk_usage
                ;;
            4)
                dry_run
                ;;
            5)
                show_disk_usage
                ;;
            6)
                print_message "${GREEN}" "Выход"
                exit 0
                ;;
            *)
                print_message "${RED}" "Неверный выбор"
                ;;
        esac

        echo ""
        read -p "Нажми Enter для продолжения..."
    done
}

# =============================================================================
# Main
# =============================================================================

if [ $# -eq 0 ]; then
    # Интерактивный режим
    interactive_menu
else
    # CLI режим
    case $1 in
        safe|1)
            show_disk_usage
            safe_cleanup
            show_disk_usage
            ;;
        moderate|2)
            show_disk_usage
            moderate_cleanup
            show_disk_usage
            ;;
        aggressive|3)
            show_disk_usage
            aggressive_cleanup
            show_disk_usage
            ;;
        dry-run)
            dry_run
            ;;
        status)
            show_disk_usage
            ;;
        help|--help|-h)
            show_help
            ;;
        *)
            print_message "${RED}" "Неизвестная команда: $1"
            echo ""
            show_help
            exit 1
            ;;
    esac
fi
