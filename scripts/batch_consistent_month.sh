#!/bin/bash
# ============================================================================
# BATCH: consistent_month - Ejecutar cada 12 horas
# Fecha: 2025-11-29
# ============================================================================

LOG_FILE="/var/log/batch_consistent_month.log"
DB_HOST="localhost"
DB_USER="avalencia"
DB_NAME="tfactu"
DB_PASS="Jacobo23"

echo "========================================" >> "$LOG_FILE"
echo "Iniciando batch_consistent_month: $(date)" >> "$LOG_FILE"

PGPASSWORD="$DB_PASS" psql -h "$DB_HOST" -U "$DB_USER" -d "$DB_NAME" -c \
    "SELECT gamification.run_batch_consistent_month_with_log();" \
    >> "$LOG_FILE" 2>&1

echo "Finalizado: $(date)" >> "$LOG_FILE"
echo "========================================" >> "$LOG_FILE"
