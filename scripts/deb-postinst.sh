#!/bin/sh
set -e

# 注册 .mytproject 文件关联的 mime 数据库索引
if command -v update-mime-database >/dev/null 2>&1; then
  update-mime-database /usr/share/mime >/dev/null 2>&1 || true
fi

exit 0