#!/bin/bash

# Get cursor position using swaymsg
CURSOR_INFO=$(swaymsg -t get_seats | jq -r '.[0].pointer')
X=$(echo "$CURSOR_INFO" | jq -r '.x' | cut -d'.' -f1)
Y=$(echo "$CURSOR_INFO" | jq -r '.y' | cut -d'.' -f1)

# Launch wlvolctl with coordinates
wlvolctl --popup --x="$X" --y="$Y"
