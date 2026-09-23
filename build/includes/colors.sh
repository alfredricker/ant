#!/usr/bin/env bash
# Shared terminal colors. Disabled automatically when not writing to a tty.
if [ -t 1 ]; then
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[0;33m'
    BLUE='\033[0;34m'
    ENDCOLOR='\033[0m'
else
    RED=''; GREEN=''; YELLOW=''; BLUE=''; ENDCOLOR=''
fi
export RED GREEN YELLOW BLUE ENDCOLOR
